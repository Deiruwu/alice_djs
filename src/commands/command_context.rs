use std::sync::Arc;
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