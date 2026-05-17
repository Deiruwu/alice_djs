use std::future::Future;
use std::pin::Pin;

use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;
use crate::audio::PlaybackStatus;

// Importamos tus helpers de UI (ajusta la ruta según cómo llamaste al módulo)
use crate::bot::{build_track_embed, build_queue_embed, TrackEmbedOptions, COLOR_PLAYING};

pub struct PlayCommand;

impl Command for PlayCommand {
    fn name(&self) -> &str { "play" }
    fn aliases(&self) -> &[&str] { &["p"] }
    fn category(&self) -> &str { "Playback" }
    fn description(&self) -> &str { "Busca y reproduce una canción." }
    fn usage(&self) -> &str { "play <búsqueda o URL>" }
    fn min_args(&self) -> usize { 1 }

    fn execute<'a>(&'a self, ctx: &'a CommandContext) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {

            // ── 1. Asunciones Seguras (Middleware) ─────────────────────────
            let guild_id = ctx.guild_id.unwrap();
            let voice_channel = ctx.voice_channel_id.unwrap();
            let query = ctx.args.join(" ");

            // ── 2. Conexión de Voz (Songbird) ──────────────────────────────
            let manager = songbird::get(&ctx.discord_ctx)
                .await
                .expect("Songbird no fue registrado en el cliente");

            let handler = match manager.join(guild_id, voice_channel).await {
                Ok(h) => h,
                Err(e) => {
                    ctx.reply(format!("Error al conectar al canal de voz: {:?}", e)).await;
                    return;
                }
            };

            // ── 3. Resolver + Encolar ──────────────────────────────────────
            match ctx.music_manager
                .resolve_and_enqueue(
                    guild_id,
                    handler,
                    &query,
                    ctx.author_name.clone(),
                    ctx.author_avatar.clone(),
                    ctx.channel_id,
                    voice_channel,
                    ctx.discord_ctx.http.clone(),
                )
                .await
            {
                Ok(status) => {
                    let author_nick = ctx.author_nick.as_deref().unwrap_or(&ctx.author_name);

                    let embed = match status {
                        PlaybackStatus::PlayingNow(track) => {
                            build_track_embed(TrackEmbedOptions {
                                track:        &track,
                                requested_by: author_nick,
                                author_icon_url: ctx.author_avatar.as_deref(),
                                position:     None,
                                color:        COLOR_PLAYING,
                                title_prefix: "🎵 Estás escuchando:",
                            })
                        },
                        PlaybackStatus::Enqueued { track, position } => {
                            build_queue_embed(
                                &track,
                                author_nick,
                                ctx.author_avatar.as_deref(),
                                position,
                            )
                        }
                    };

                    ctx.reply_embed(embed).await;
                }

                Err(e) => {
                    eprintln!("[ERROR] play: {}", e);
                }
            }
        })
    }
}