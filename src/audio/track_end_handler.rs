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
    pub scheduler:        Arc<Mutex<TrackScheduler>>,
    pub music_manager:    Arc<MusicManager>,
    pub guild_id:         GuildId,
    pub channel_id:       ChannelId,
    pub voice_channel_id: Option<ChannelId>,
    pub http:             Arc<Http>,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<songbird::Event> {
        println!("[TrackEnd] Evento disparado para guild {}", self.guild_id);

        let mut scheduler = self.scheduler.lock().await;

        scheduler.advance_logical_queue();

        if scheduler.current.is_none() {
            println!("[TrackEnd] La cola está vacía. Terminando reproducción.");
            return None;
        }

        scheduler.play_current(None).await;

        if let Some(playing_now) = scheduler.current.clone() {

            drop(scheduler);

            let topic = format!("🎶 {} — Request by {}", playing_now.track.title, playing_now.requested_by);

            if let Some(vc_id) = self.voice_channel_id {
                match vc_id.edit(&self.http, EditChannel::new().status(topic)).await {
                    Ok(_) => println!("[TrackEnd] Tópico actualizado con éxito."),
                    Err(e) => eprintln!("[TrackEnd] Error editando tópico de voz: {:?}", e),
                }
            }

            let embed = build_track_embed(TrackEmbedOptions {
                track:           &playing_now.track,
                requested_by:    &playing_now.requested_by,
                author_icon_url: playing_now.requester_avatar.as_deref(),
                position:        None,
                color:           COLOR_PLAYING,
                title_prefix:    "🎵 Estás escuchando:",
            });

            if let Err(e) = self.channel_id.send_message(&self.http, CreateMessage::new().embed(embed)).await {
                eprintln!("[TrackEnd] Error al enviar el embed: {}", e);
            }
        }

        None
    }
}