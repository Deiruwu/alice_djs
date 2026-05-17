use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use songbird::Call;

use crate::config::Config;
use crate::microservices::MicroserviceClient;
use crate::model::{Track, TrackState};
use super::track_scheduler::{QueuedTrack, TrackScheduler};
use super::errors::MusicError;
use super::encoder::create_input;
use super::PlaybackStatus;

// ─── Music Manager ───────────────────────────────────────────────────────

pub struct MusicManager {
    config: Arc<Config>,
    pub client: MicroserviceClient,
    schedulers: Mutex<HashMap<GuildId, Arc<Mutex<TrackScheduler>>>>,
}

impl MusicManager {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            client: MicroserviceClient::new(&config),
            config,
            schedulers: Mutex::new(HashMap::new()),
        }
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /// Público para que TrackScheduler y TrackEndHandler puedan reconstruir el Input de forma diferida (JIT).
    pub fn build_input_public(&self, track: &Track) -> Result<songbird::input::Input, MusicError> {
        let file_path = track.file_path.clone().ok_or(MusicError::NoFilePath)?;
        let effective_path = self.config.resolve_path(&file_path);
        create_input(&effective_path)
            .map_err(|e| MusicError::EncoderError(e.to_string()))
    }

    /// Corrección asíncrona pura: Eliminamos `block_in_place` y la API de `.entry()`.
    /// Usamos un flujo secuencial asíncrono para inicializar el Scheduler de forma segura.
    pub async fn get_or_create_scheduler(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        channel_id: ChannelId,
        voice_channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Arc<Mutex<TrackScheduler>> {
        let mut schedulers = self.schedulers.lock().await;

        if !schedulers.contains_key(&guild_id) {
            let scheduler = TrackScheduler::new(call);
            let arc = Arc::new(Mutex::new(scheduler));

            {
                let mut s = arc.lock().await;
                s.self_arc         = Some(arc.clone());
                s.music_manager    = Some(self.clone());
                s.voice_channel_id = Some(voice_channel_id);
                s.guild_id         = Some(guild_id);
            }

            schedulers.insert(guild_id, arc);
        }

        let arc = schedulers.get(&guild_id).unwrap().clone();

        {
            let mut s = arc.lock().await;
            s.channel_id = Some(channel_id);
            s.http       = Some(http);
        }

        arc
    }

    // ─── Reproducción (Arquitectura JIT) ─────────────────────────────────────

    pub async fn resolve_and_enqueue(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        query: &str,
        requested_by: String,
        requester_avatar: Option<String>,
        channel_id: ChannelId,
        voice_channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Result<PlaybackStatus, MusicError> {
        let track = self.client
            .resolve(query)
            .await
            .map_err(|e| MusicError::ResolveError(e.to_string()))?;

        let scheduler_arc = self.get_or_create_scheduler(guild_id, call, channel_id, voice_channel_id, http).await;
        let mut scheduler = scheduler_arc.lock().await;

        let is_empty = scheduler.current.is_none();

        let audio_input = if is_empty {
            Some(self.build_input_public(&track)?)
        } else {
            None
        };

        let status = if is_empty {
            PlaybackStatus::PlayingNow(track.clone())
        } else {
            PlaybackStatus::Enqueued {
                track: track.clone(),
                position: scheduler.queue.len() + 1,
            }
        };

        scheduler.enqueue(track, requested_by, requester_avatar, audio_input).await;

        Ok(status)
    }

    pub async fn resolve_and_enqueue_next(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        query: &str,
        requested_by: String,
        requester_avatar: Option<String>,
        channel_id: ChannelId,
        voice_channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Result<PlaybackStatus, MusicError> {
        let track = self.client
            .resolve(query)
            .await
            .map_err(|e| MusicError::ResolveError(e.to_string()))?;

        let scheduler_arc = self.get_or_create_scheduler(guild_id, call, channel_id, voice_channel_id, http).await;
        let mut scheduler = scheduler_arc.lock().await;

        let is_empty = scheduler.current.is_none();

        let audio_input = if is_empty {
            Some(self.build_input_public(&track)?)
        } else {
            None
        };

        let status = if is_empty {
            PlaybackStatus::PlayingNow(track.clone())
        } else {
            PlaybackStatus::Enqueued {
                track: track.clone(),
                position: 1,
            }
        };

        scheduler.enqueue_next(track, requested_by, requester_avatar, audio_input).await;

        Ok(status)
    }

    pub async fn resolve_and_enqueue_radio(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        query: &str,
        requested_by: String,
        requester_avatar: Option<String>,
        channel_id: ChannelId,
        voice_channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Result<(usize, Track), MusicError> {

        let seed_track = self.client
            .resolve(query)
            .await
            .map_err(|e| MusicError::ResolveError(format!("Fallo al buscar la semilla: {}", e)))?;

        let mut tracks = self.client
            .radio(&seed_track.id)
            .await
            .map_err(|e| MusicError::ResolveError(format!("Fallo al generar mix: {}", e)))?;

        if tracks.is_empty() {
            return Err(MusicError::ResolveError("La radio no devolvió resultados para esta semilla.".into()));
        }

        // Validación defensiva: Evita duplicar la semilla si el microservicio ya la incluyó en el índice [0]
        let seed_already_first = tracks.first().map(|t| t.id.as_str()) == Some(seed_track.id.as_str());
        if !seed_already_first {
            tracks.insert(0, seed_track.clone());
        }

        let scheduler_arc = self.get_or_create_scheduler(guild_id, call, channel_id, voice_channel_id, http).await;
        let mut scheduler = scheduler_arc.lock().await;

        let mut enqueued_count = 0;

        for (i, mut track) in tracks.into_iter().enumerate() {
            let is_first_and_empty = scheduler.current.is_none() && i == 0;

            if is_first_and_empty && track.state == TrackState::Partial {
                track = self.client.resolve(&track.id).await
                    .map_err(|e| MusicError::ResolveError(e.to_string()))?;
            }

            let audio_input = if is_first_and_empty {
                Some(self.build_input_public(&track)?)
            } else {
                None
            };

            scheduler.enqueue(track, requested_by.clone(), requester_avatar.clone(), audio_input).await;
            enqueued_count += 1;
        }

        Ok((enqueued_count, seed_track))
    }

    // ─── Control de cola ─────────────────────────────────────────────────────

    pub async fn clear_queue(&self, guild_id: GuildId) -> Result<(), MusicError> {
        let schedulers = self.schedulers.lock().await;
        let scheduler_arc = schedulers.get(&guild_id).ok_or(MusicError::NotPlaying)?;
        let mut scheduler = scheduler_arc.lock().await;
        scheduler.clear().await;
        Ok(())
    }

    pub async fn skip(&self, guild_id: GuildId) -> Result<(), MusicError> {
        let schedulers = self.schedulers.lock().await;
        let scheduler_arc = schedulers.get(&guild_id).ok_or(MusicError::NotPlaying)?;
        let mut scheduler = scheduler_arc.lock().await;
        scheduler.skip().await;
        Ok(())
    }

    pub async fn skip_to(&self, guild_id: GuildId, index: usize) -> Result<QueuedTrack, MusicError> {
        let schedulers = self.schedulers.lock().await;
        let scheduler_arc = schedulers.get(&guild_id).ok_or(MusicError::NotPlaying)?;
        let mut scheduler = scheduler_arc.lock().await;
        scheduler.skip_to(index).await
    }

    pub async fn set_volume(&self, guild_id: GuildId, level: u8) -> Result<(), MusicError> {
        let schedulers = self.schedulers.lock().await;
        let scheduler_arc = schedulers.get(&guild_id).ok_or(MusicError::NotPlaying)?;
        let mut scheduler = scheduler_arc.lock().await;
        scheduler.set_volume(level).await
    }

    pub async fn get_volume(&self, guild_id: GuildId) -> Option<u8> {
        let schedulers = self.schedulers.lock().await;
        let scheduler_arc = schedulers.get(&guild_id)?;
        let scheduler = scheduler_arc.lock().await;
        Some(scheduler.current_volume_percent())
    }

    pub async fn get_queue(
        &self,
        guild_id: GuildId,
    ) -> Option<(Option<QueuedTrack>, Vec<QueuedTrack>)> {
        let schedulers = self.schedulers.lock().await;
        let scheduler_arc = schedulers.get(&guild_id)?;
        let scheduler = scheduler_arc.lock().await;
        Some((
            scheduler.current.clone(),
            scheduler.queue.iter().cloned().collect(),
        ))
    }
    pub async fn shutdown_all(&self) {
        let arcs: Vec<_> = {
            let schedulers = self.schedulers.lock().await;
            schedulers.values().cloned().collect()
        };

        for scheduler_arc in arcs {
            let mut scheduler = scheduler_arc.lock().await;
            scheduler.clear().await;
        }
    }
}