use std::sync::Arc;
use serenity::all::{CreateEmbed, CreateMessage};
use serenity::client::Context;
use serenity::model::id::{ChannelId, GuildId, UserId};
use crate::audio::MusicManager;

#[derive(Clone)]
pub struct CommandContext {
    pub discord_ctx: Context,
    pub music_manager: Arc<MusicManager>,
    pub author_id: UserId,
    pub author_name: String,
    pub author_nick: Option<String>,
    pub author_avatar: Option<String>,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub voice_channel_id: Option<ChannelId>,
    pub args: Vec<String>,
}

impl std::fmt::Debug for CommandContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandContext")
            .field("author_name", &self.author_name)
            .field("args", &self.args)
            .finish()
    }

}

impl CommandContext {
    pub async fn reply(&self, content: impl Into<String>) {
        if let Err(e) = self.channel_id.say(&self.discord_ctx.http, content).await {
            eprintln!("[ERROR] Fallo al responder en {}: {}", self.channel_id, e);
        }
    }

    pub async fn reply_usage(&self, usage: &str) {
        self.reply(format!("Usa: `{}`", usage)).await;
    }

    pub async fn reply_embed(&self, embed: CreateEmbed) {
        if let Err(e) = self.channel_id
            .send_message(&self.discord_ctx.http, CreateMessage::new().embed(embed))
            .await {
            eprintln!("[ERROR] Fallo al enviar embed: {}", e);
        }
    }
}