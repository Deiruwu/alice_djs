use std::sync::Arc;
use tokio::sync::Mutex;
use serenity::builder::{CreateMessage, EditChannel};
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use songbird::EventContext;
use songbird::EventHandler;
use async_trait::async_trait;

use super::track_scheduler::TrackScheduler;
use super::music_manager::MusicManager;
use crate::bot::*;

pub struct TrackEndHandler {
    pub scheduler:     Arc<Mutex<TrackScheduler>>,
    pub music_manager: Arc<MusicManager>,
    pub guild_id:      GuildId,
    pub channel_id:    ChannelId, // Este debe ser el ID del canal de TEXTO
    pub voice_channel_id: Option<ChannelId>,
    pub http:          Arc<Http>,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<songbird::Event> {
        println!("[TrackEnd] Evento disparado para guild {}", self.guild_id);

        let mut scheduler = self.scheduler.lock().await;
        scheduler.advance_logical_queue();

        let next = scheduler.current.clone();
        println!("[TrackEnd] Avanzando cola. Ahora suena: {:?}",
                 next.as_ref().map(|t| &t.track.title));

        if let Some(queued) = next {
            drop(scheduler);

            match self.music_manager.build_input_public(&queued.track) {
                Ok(audio_input) => {
                    let mut scheduler = self.scheduler.lock().await;
                    scheduler.play_current(audio_input).await;
                    drop(scheduler);

                    // ── Intento de edición con Debug de Permisos ──
                    let topic = format!("🎶 {} — Request by {}", queued.track.title, queued.requested_by);

                    // Intentamos la edición
                    match self.voice_channel_id.unwrap().edit(&self.http, EditChannel::new().status(topic)).await {
                        Ok(_) => println!("[TrackEnd] Tópico actualizado con éxito."),
                        Err(e) => {
                            eprintln!("[TrackEnd] Error editando tópico: {:?}", e);
                        }
                    }

                    // ── Envío del Embed ──
                    let embed = build_track_embed(TrackEmbedOptions {
                        track:           &queued.track,
                        requested_by:    &queued.requested_by,
                        author_icon_url: queued.requester_avatar.as_deref(),
                        position:        None,
                        color:           COLOR_PLAYING,
                        title_prefix:    "🎵 Estás escuchando:",
                    });

                    let _ = self.channel_id.send_message(&self.http, CreateMessage::new().embed(embed)).await;
                }
                Err(e) => eprintln!("[TrackEnd] Error al procesar audio: {}", e),
            }
        }
        None
    }
}