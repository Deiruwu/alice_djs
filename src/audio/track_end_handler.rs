use std::sync::Arc;
use tokio::sync::Mutex;
use serenity::builder::CreateMessage;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use songbird::EventContext;
use songbird::EventHandler;
use async_trait::async_trait;
use super::track_scheduler::TrackScheduler;
use super::music_manager::MusicManager;
use serenity::model::id::GuildId;
use crate::bot::*;

pub struct TrackEndHandler {
    pub scheduler:     Arc<Mutex<TrackScheduler>>,
    pub music_manager: Arc<MusicManager>,
    pub guild_id:      GuildId,
    /// Canal donde se mandan los mensajes de "ahora suena".
    pub channel_id:    ChannelId,
    pub http:          Arc<Http>,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<songbird::Event> {
        let mut scheduler = self.scheduler.lock().await;
        scheduler.advance_logical_queue();

        let next = scheduler.current.clone();
        println!("[TrackEnd] Avanzando cola. Ahora suena: {:?}",
                 next.as_ref().map(|t| &t.track.title));

        if let Some(queued) = next {
            drop(scheduler); // soltamos antes del I/O

            match self.music_manager.build_input_public(&queued.track) {
                Ok(audio_input) => {
                    let mut scheduler = self.scheduler.lock().await;
                    scheduler.play_current(audio_input).await;
                    drop(scheduler);

                    // ── Embed "ahora suena" ───────────────────────────────────
                    let embed = build_track_embed(TrackEmbedOptions {
                        track:        &queued.track,
                        requested_by: &queued.requested_by,
                        position:     None,
                        color:        COLOR_PLAYING,
                        title_prefix: "🎵 Ahora suena",
                    });

                    let _ = self.channel_id
                        .send_message(&self.http, CreateMessage::new().embed(embed))
                        .await;
                }
                Err(e) => {
                    eprintln!("[TrackEnd] Error construyendo input para '{}': {}",
                              queued.track.title, e);
                }
            }
        }

        None
    }
}