use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use songbird::{Call, Event, TrackEvent, input::Input};
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use crate::model::Track;
use super::errors::MusicError;
use super::track_end_handler::TrackEndHandler;
use super::music_manager::MusicManager;

#[derive(Debug, Clone)]
pub struct QueuedTrack {
    pub track: Track,
    pub requested_by: String,
}

pub struct TrackScheduler {
    pub queue:         VecDeque<QueuedTrack>,
    pub current:       Option<QueuedTrack>,
    volume:            f32,
    call:              Arc<Mutex<Call>>,
    pub self_arc:      Option<Arc<Mutex<TrackScheduler>>>,
    /// Referencia al MusicManager para reconstruir Input en TrackEndHandler.
    pub music_manager: Option<Arc<MusicManager>>,
    pub guild_id:      Option<GuildId>,
    /// Canal de texto donde se envían los embeds de "ahora suena".
    pub channel_id:    Option<ChannelId>,
    pub http:          Option<Arc<Http>>,
}

impl TrackScheduler {
    pub fn new(call: Arc<Mutex<Call>>) -> Self {
        Self {
            queue:         VecDeque::new(),
            current:       None,
            volume:        1.0,
            call,
            self_arc:      None,
            music_manager: None,
            guild_id:      None,
            channel_id:    None,
            http:          None,
        }
    }

    // ─── Reproducir current en Songbird ──────────────────────────────────────
    // Limpia la cola del driver y reproduce solo la pista actual.
    // Es el único punto donde se llama a driver.play_input().

    pub async fn play_current(&mut self, audio_input: Input) {
        let mut driver = self.call.lock().await;

        // Limpiamos cualquier basura que haya quedado en la cola del driver.
        driver.queue().modify_queue(|q| q.clear());

        let handle = driver.enqueue_input(audio_input).await;
        let _ = handle.set_volume(self.volume);

        if let (Some(scheduler_arc), Some(manager), Some(guild_id), Some(channel_id), Some(http)) =
            (&self.self_arc, &self.music_manager, &self.guild_id, &self.channel_id, &self.http)
        {
            let _ = handle.add_event(
                Event::Track(TrackEvent::End),
                TrackEndHandler {
                    scheduler:     scheduler_arc.clone(),
                    music_manager: manager.clone(),
                    guild_id:      *guild_id,
                    channel_id:    *channel_id,
                    http:          http.clone(),
                },
            );
        }

        println!("[play_current] Reproduciendo: {:?}",
                 self.current.as_ref().map(|t| &t.track.title));
    }

    // ─── Encolar al final ────────────────────────────────────────────────────

    pub async fn enqueue(&mut self, track: Track, requested_by: String, audio_input: Input) {
        let queued = QueuedTrack { track, requested_by };
        let was_idle = self.current.is_none();

        self.queue.push_back(queued);

        if was_idle {
            self.advance_logical_queue();
            self.play_current(audio_input).await;
        }
        // Si ya hay algo sonando, solo encolamos lógicamente.
        // El TrackEndHandler llamará a advance_logical_queue() + play_current()
        // cuando llegue el momento — pero para eso necesitamos el audio_input
        // guardado. Ver nota abajo sobre InputStore.
    }

    // ─── Encolar con prioridad (al frente) ───────────────────────────────────

    pub async fn enqueue_next(&mut self, track: Track, requested_by: String, audio_input: Input) {
        let queued = QueuedTrack { track, requested_by };

        if self.current.is_none() {
            // No hay nada sonando — reproducir directamente.
            self.queue.push_front(queued);
            self.advance_logical_queue();
            self.play_current(audio_input).await;
        } else {
            // Hay algo sonando — solo insertamos al frente de la cola lógica.
            // NO interrumpimos la pista actual ni llamamos play_current.
            // El audio_input lo descartamos: el TrackEndHandler reconstruirá
            // el Input desde MusicManager (build_input_public) cuando sea el turno.
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
        // stop() en la pista actual dispara TrackEnd → advance_logical_queue()
        if let Some(current) = driver.queue().current() {
            let _ = current.stop();
        }
    }

    // ─── Skip hasta posición ─────────────────────────────────────────────────

    pub async fn skip_to(&mut self, index: usize) -> Result<QueuedTrack, MusicError> {
        if index >= self.queue.len() {
            return Err(MusicError::OutOfRange);
        }

        // Descartamos tracks intermedios de la cola lógica.
        for _ in 0..index {
            self.queue.pop_front();
        }

        // Paramos la pista actual; el TrackEnd handler avanzará la cola.
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