use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use songbird::Call;
use crate::config::Config;
use crate::microservices::MicroserviceClient;
use crate::model::Track;
use super::track_scheduler::{QueuedTrack, TrackScheduler};
use super::errors::MusicError;
use super::encoder::create_input;
use super::PlaybackStatus;

// ─── Music Manager ───────────────────────────────────────────────────────

pub struct MusicManager {
    config: Arc<Config>,
    client: MicroserviceClient,
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

    /// Público para que TrackEndHandler pueda reconstruir el Input.
    pub fn build_input_public(&self, track: &Track) -> Result<songbird::input::Input, MusicError> {
        let file_path = track.file_path.clone().ok_or(MusicError::NoFilePath)?;
        let effective_path = self.config.resolve_path(&file_path);
        create_input(&effective_path)
            .map_err(|e| MusicError::EncoderError(e.to_string()))
    }

    pub async fn get_or_create_scheduler(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Arc<Mutex<TrackScheduler>> {
        let mut schedulers = self.schedulers.lock().await;
        let manager_arc = self.clone();

        let arc = schedulers
            .entry(guild_id)
            .or_insert_with(|| {
                let scheduler = TrackScheduler::new(call);
                let arc = Arc::new(Mutex::new(scheduler));
                let arc_clone = arc.clone();

                tokio::task::block_in_place(|| {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        let mut s = arc_clone.lock().await;
                        s.self_arc        = Some(arc_clone.clone());
                        s.music_manager   = Some(manager_arc.clone());
                        s.guild_id        = Some(guild_id);
                    });
                });
                arc
            })
            .clone();

        // Actualizamos siempre channel_id y http por si el comando se llamó
        // desde otro canal en una sesión posterior.
        {
            let mut s = arc.lock().await;
            s.channel_id = Some(channel_id);
            s.http       = Some(http);
        }
        arc
    }

    // ─── Reproducción ────────────────────────────────────────────────────────

    pub async fn resolve_and_enqueue(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        query: &str,
        requested_by: String,
        channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Result<PlaybackStatus, MusicError> {
        let track = self.client
            .resolve(query)
            .await
            .map_err(|e| MusicError::ResolveError(e.to_string()))?;

        let audio_input = self.build_input_public(&track)?;

        let scheduler_arc = self.get_or_create_scheduler(guild_id, call, channel_id, http).await;
        let mut scheduler = scheduler_arc.lock().await;

        // Evaluamos el estado antes de mutar la cola
        let status = if scheduler.current.is_none() {
            PlaybackStatus::PlayingNow(track.clone())
        } else {
            PlaybackStatus::Enqueued {
                track: track.clone(),
                position: scheduler.queue.len() + 1,
            }
        };

        scheduler.enqueue(track, requested_by, audio_input).await;

        Ok(status)
    }

    pub async fn resolve_and_enqueue_next(
        self: &Arc<Self>,
        guild_id: GuildId,
        call: Arc<Mutex<Call>>,
        query: &str,
        requested_by: String,
        channel_id: ChannelId,
        http: Arc<Http>,
    ) -> Result<PlaybackStatus, MusicError> {
        let track = self.client
            .resolve(query)
            .await
            .map_err(|e| MusicError::ResolveError(e.to_string()))?;

        let audio_input = self.build_input_public(&track)?;

        let scheduler_arc = self.get_or_create_scheduler(guild_id, call, channel_id, http).await;
        let mut scheduler = scheduler_arc.lock().await;

        // Evaluamos el estado antes de mutar la cola
        let status = if scheduler.current.is_none() {
            PlaybackStatus::PlayingNow(track.clone())
        } else {
            PlaybackStatus::Enqueued {
                track: track.clone(),
                position: 1, // Prioridad absoluta: va al frente de la cola de espera
            }
        };

        scheduler.enqueue_next(track, requested_by, audio_input).await;

        Ok(status)
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
}