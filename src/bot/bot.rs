use std::sync::Arc;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, voice::VoiceState},
    prelude::*,
};

use crate::audio::MusicManager;
use crate::commands::{
    CommandContext,
    CommandRegistry,
};

const PREFIX: &str = "t!";

/// Capa de eventos de Discord.
///
/// Su unica responsabilidad es traducir eventos de serenity en tipos
/// propios del bot (CommandContext) y delegarlos al registry.
/// No contiene logica de comandos ni de audio.
pub struct Bot {
    pub registry:      Arc<CommandRegistry>,
    pub music_manager: Arc<MusicManager>,
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let content = match msg.content.strip_prefix(PREFIX) {
            Some(c) => c.trim(),
            None => return,
        };

        let mut parts = content.splitn(2, ' ');
        let label = match parts.next() {
            Some(l) if !l.is_empty() => l.to_lowercase(),
            _ => return,
        };
        let args: Vec<String> = parts
            .next()
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_string)
            .collect();

        let voice_channel_id = {
            msg.guild_id.and_then(|gid| {
                let guild = ctx.cache.guild(gid)?;
                guild.voice_states.get(&msg.author.id)?.channel_id
            })
        };

        let command_ctx = CommandContext {
            discord_ctx:      ctx,
            music_manager:    Arc::clone(&self.music_manager),
            author_id:        msg.author.id,
            author_name:      msg.author.name.clone(),
            channel_id:       msg.channel_id,
            guild_id:         msg.guild_id,
            voice_channel_id,
            args,
        };

        self.registry.dispatch(&label, &command_ctx).await;
    }
}