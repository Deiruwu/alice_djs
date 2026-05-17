use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use songbird::{Call, Event, TrackEvent, input::Input};
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};

use crate::model::{Track, TrackState};
use super::errors::MusicError;
use super::track_end_handler::TrackEndHandler;
use super::music_manager::MusicManager;

#[derive(Debug, Clone)]
pub struct QueuedTrack {
    pub track: Track,
    pub requested_by: String,
    pub requester_avatar: Option<String>,
}

pub struct TrackScheduler {
    pub queue:            VecDeque<QueuedTrack>,
    pub current:          Option<QueuedTrack>,
    volume:               f32,
    call:                 Arc<Mutex<Call>>,
    pub self_arc:         Option<Arc<Mutex<TrackScheduler>>>,
    pub music_manager:    Option<Arc<MusicManager>>,
    pub guild_id:         Option<GuildId>,
    pub channel_id:       Option<ChannelId>,
    pub voice_channel_id: Option<ChannelId>,
    pub http:             Option<Arc<Http>>,
}

impl TrackScheduler {
    pub fn new(call: Arc<Mutex<Call>>) -> Self {
        Self {
            queue:            VecDeque::new(),
            current:          None,
            volume:           1.0,
            call,
            self_arc:         None,
            music_manager:    None,
            guild_id:         None,
            channel_id:       None,
            voice_channel_id: None,
            http:             None,
        }
    }

    // ─── Reproducir current en Songbird ──────────────────────────────────────

    pub async fn play_current(&mut self, audio_input: Option<Input>) {
        // ── LÓGICA JIT (Just-In-Time) ──
        let input_to_play = match audio_input {
            Some(input) => input, // Fast path: Si ya nos dieron el audio, lo usamos directo.
            None => {
                // Fallback: No hay audio preparado. Hay que construirlo ahora mismo.
                let mut current_qt = match self.current.clone() {
                    Some(qt) => qt,
                    None => return, // No hay nada en current, abortamos.
                };

                let manager = match &self.music_manager {
                    Some(m) => m.clone(),
                    None => {
                        eprintln!("[FATAL] MusicManager no está enlazado al scheduler.");
                        return;
                    }
                };

                // 1. Resolución Perezosa: Si es 'partial', obligamos a Python a descargarla AHORA.
                if current_qt.track.state == TrackState::Partial {
                    println!("[JIT] Resolviendo track parcial: {}", current_qt.track.title);

                    match manager.client.resolve(&current_qt.track.id).await {
                        Ok(resolved_track) => {
                            // Sobrescribimos el track parcial con el track completo (que ya tiene file_path)
                            current_qt.track = resolved_track;
                            // Actualizamos el estado interno para que los comandos !queue o !nowplaying vean los datos reales.
                            self.current = Some(current_qt.clone());
                        }
                        Err(e) => {
                            eprintln!("[ERROR] Falló la resolución JIT de '{}': {}", current_qt.track.title, e);
                            // Si falla la descarga, saltamos la canción para no trabar la cola.
                            self.advance_logical_queue();
                            // Podríamos hacer un auto-skip recursivo aquí, pero por seguridad retornamos.
                            return;
                        }
                    }
                }

                // 2. Ahora que garantizamos que es 'cached', abrimos el File Descriptor
                match manager.build_input_public(&current_qt.track) {
                    Ok(input) => input,
                    Err(e) => {
                        eprintln!("[ERROR] Falló build_input_public en JIT: {}", e);
                        self.advance_logical_queue();
                        return;
                    }
                }
            }
        };

        // ── INYECCIÓN EN SONGBIRD ──
        let mut driver = self.call.lock().await;

        driver.queue().modify_queue(|q| q.clear());

        let handle = driver.enqueue_input(input_to_play).await;
        let _ = handle.set_volume(self.volume);

        if let (Some(scheduler_arc), Some(manager), Some(guild_id), Some(channel_id), Some(http)) =
            (&self.self_arc, &self.music_manager, &self.guild_id, &self.channel_id, &self.http)
        {
            let _ = handle.add_event(
                Event::Track(TrackEvent::End),
                TrackEndHandler {
                    scheduler:        scheduler_arc.clone(),
                    music_manager:    manager.clone(),
                    guild_id:         *guild_id,
                    channel_id:       *channel_id,
                    voice_channel_id: self.voice_channel_id,
                    http:             http.clone(),
                },
            );
        }

        println!("[play_current] Reproduciendo: {:?}",
                 self.current.as_ref().map(|t| &t.track.title));

        // ── REGISTRO DE MÉTRICAS (Fire and Forget) ──
        if let (Some(manager), Some(playing_now)) = (&self.music_manager, &self.current) {
            let track_id = playing_now.track.id.clone();
            let manager_clone = manager.clone();

            tokio::spawn(async move {
                if let Err(e) = manager_clone.client.mark_as_played(&track_id).await {
                    eprintln!("[STATS] Error al sumar reproducción para '{}': {}", track_id, e);
                } else {
                    println!("[STATS] +1 Play registrado para: {}", track_id);
                }
            });
        }
    }

    // ─── Encolar al final ────────────────────────────────────────────────────

    pub async fn enqueue(&mut self, track: Track, requested_by: String, requester_avatar: Option<String>, audio_input: Option<Input>) {
        let queued = QueuedTrack { track, requested_by, requester_avatar };
        let was_idle = self.current.is_none();

        self.queue.push_back(queued);

        if was_idle {
            self.advance_logical_queue();
            self.play_current(audio_input).await;
        }
    }

    // ─── Encolar con prioridad (al frente) ───────────────────────────────────

    pub async fn enqueue_next(&mut self, track: Track, requested_by: String, requester_avatar: Option<String>, audio_input: Option<Input>) {
        let queued = QueuedTrack { track, requested_by, requester_avatar };

        if self.current.is_none() {
            self.queue.push_front(queued);
            self.advance_logical_queue();
            self.play_current(audio_input).await;
        } else {
            // Descartamos el audio_input si venía algo, para ahorrar memoria.
            drop(audio_input);
            self.queue.push_front(queued);
            println!("[enqueue_next] Encolado con prioridad: '{:?}'. Sonará después de la pista actual.",
                     self.queue.front().map(|t| &t.track.title));
        }
    }

    // ─── Avanzar cola lógica ─────────────────────────────────────────────────

    pub fn advance_logical_queue(&mut self) {
        self.current = self.queue.pop_front();
    }

    // ─── Skip simple ─────────────────────────────────────────────────────────

    pub async fn skip(&mut self) {
        let driver = self.call.lock().await;
        if let Some(current) = driver.queue().current() {
            let _ = current.stop();
        }
    }

    // ─── Skip hasta posición ─────────────────────────────────────────────────

    pub async fn skip_to(&mut self, index: usize) -> Result<QueuedTrack, MusicError> {
        if index >= self.queue.len() {
            return Err(MusicError::OutOfRange);
        }

        for _ in 0..index {
            self.queue.pop_front();
        }

        let driver = self.call.lock().await;
        if let Some(current) = driver.queue().current() {
            let _ = current.stop();
        }

        self.queue.front().cloned().ok_or(MusicError::NotPlaying)
    }

    // ─── Limpiar cola ────────────────────────────────────────────────────────

    pub async fn clear(&mut self) {
        self.queue.clear();
        self.current = None;
        let mut driver = self.call.lock().await;
        driver.queue().modify_queue(|q| q.clear());
        driver.stop();
    }

    // ─── Volumen ─────────────────────────────────────────────────────────────

    pub async fn set_volume(&mut self, level: u8) -> Result<(), MusicError> {
        let volume = (level as f32) / 100.0;
        self.volume = volume;
        let driver = self.call.lock().await;
        if let Some(handle) = driver.queue().current() {
            let _ = handle.set_volume(volume);
        }
        Ok(())
    }

    pub fn current_volume_percent(&self) -> u8 {
        (self.volume * 100.0).round() as u8
    }
}