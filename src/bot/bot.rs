use std::sync::Arc;
use std::time::Duration;

use serenity::{
    async_trait,
    model::{channel::Message, voice::VoiceState},
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
    // ─── Despacho de Comandos ────────────────────────────────────────────────
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

        // ─── Resolución de Identidad ──────────────────────────────────────────────
        let member = match msg.guild_id {
            Some(gid) => gid.member(&ctx.http, msg.author.id).await.ok(),
            None => None,
        };

        // Nick: preferir el del servidor, caer al global_name, caer al username
        let author_nick = member
            .as_ref()
            .and_then(|m| m.nick.clone())
            .or_else(|| msg.author.global_name.clone())
            .unwrap_or_else(|| msg.author.name.clone());

        // Avatar: preferir el del servidor, caer al global
        let author_avatar = member
            .as_ref()
            .and_then(|m| {
                // Avatar de servidor viene en el Member, no en msg.member (que puede ser parcial)
                m.avatar.as_ref().map(|hash| {
                    let hash_str = hash.to_string();
                    let ext = if hash_str.starts_with("a_") { "gif" } else { "webp" };
                    format!(
                        "https://cdn.discordapp.com/guilds/{}/users/{}/avatars/{}.{}?size=1024",
                        m.guild_id,
                        msg.author.id,
                        hash_str,
                        ext
                    )
                })
            })
            .or_else(|| msg.author.avatar_url());

        // ─── Despacho ─────────────────────────────────────────────────────────────
        let command_ctx = CommandContext {
            discord_ctx:      ctx,
            music_manager:    Arc::clone(&self.music_manager),
            author_id:        msg.author.id,
            author_name:      author_nick,
            author_nick:      member.as_ref().and_then(|m| m.nick.clone()),
            author_avatar,
            channel_id:       msg.channel_id,
            guild_id:         msg.guild_id,
            voice_channel_id,
            args,
        };

        self.registry.dispatch(&label, &command_ctx).await;
    }

    // ─── AUTO-DISCONNECT (Lazy Leave) ────────────────────────────────────────
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        // 1. Obtenemos el guild_id del evento
        let guild_id = match new.guild_id.or(old.as_ref().and_then(|o| o.guild_id)) {
            Some(id) => id,
            None => return,
        };

        // 2. Extraemos Songbird y verificamos si el bot está conectado ahí
        let manager = songbird::get(&ctx).await.expect("Songbird no registrado");
        let call_lock = match manager.get(guild_id) {
            Some(call) => call,
            None => return, // El bot no está en ningún canal de voz de este server
        };

        let bot_channel_id = {
            let call = call_lock.lock().await;
            match call.current_channel() {
                Some(id) => serenity::model::id::ChannelId::new(id.0.get()),
                None => return,
            }
        };

        // 3. Revisamos el estado de los usuarios en la caché
        let guild = match ctx.cache.guild(guild_id) {
            Some(g) => g.clone(),
            None => return,
        };

        let current_user_id = ctx.cache.current_user().id;

        // Contamos cuántos humanos hay en el mismo canal que el bot
        let humans_in_channel = guild.voice_states.values().filter(|vs| {
            vs.channel_id == Some(bot_channel_id) &&
                vs.user_id != current_user_id &&
                !vs.member.as_ref().map_or(false, |m| m.user.bot)
        }).count();

        // 4. Si el canal quedó vacío, iniciamos la cuenta regresiva
        if humans_in_channel == 0 {
            let manager_clone = manager.clone();
            let music_manager_clone = self.music_manager.clone();
            let ctx_cache = ctx.cache.clone();

            tokio::spawn(async move {
                // Tolerancia de 30 segundos
                tokio::time::sleep(Duration::from_secs(30)).await;

                // ── LA MAGIA DEL SCOPE ──
                // Abrimos un bloque de let para que `guild_recheck` nazca y muera aquí mismo.
                let humans_still_zero = if let Some(guild_recheck) = ctx_cache.guild(guild_id) {
                    guild_recheck.voice_states.values().filter(|vs| {
                        vs.channel_id == Some(bot_channel_id) &&
                            vs.user_id != current_user_id &&
                            !vs.member.as_ref().map_or(false, |m| m.user.bot)
                    }).count() == 0
                } else {
                    false
                }; // <-- Aquí se destruye la referencia de la caché. Ya es seguro usar .await.

                if humans_still_zero {
                    println!("[AUTO-LEAVE] Canal vacío por 30s en guild {}. Limpiando recursos.", guild_id);

                    let _ = manager_clone.remove(guild_id).await;
                    let _ = music_manager_clone.clear_queue(guild_id).await;
                }
            });
        }
    }
}