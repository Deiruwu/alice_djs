use std::future::Future;
use std::pin::Pin;

use serenity::builder::CreateMessage;

use crate::bot::*;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;
use crate::audio::PlaybackStatus;

pub struct PlayCommand;

impl AsyncCommand for PlayCommand {
    fn name(&self) -> &str { "play" }

    fn aliases(&self) -> &[&str] { &["p"] }

    fn category(&self) -> &str { "Música" }

    fn description(&self) -> &str {
        "Busca y reproduce una canción."
    }

    fn usage(&self) -> &str {
        "play <búsqueda o URL>"
    }

    fn execute_async<'a>(
        &'a self,
        ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {

            // ── Validaciones ────────────────────────────────────────────────

            let guild_id = match ctx.guild_id {
                Some(id) => id,
                None => {
                    let _ = ctx.channel_id
                        .say(
                            &ctx.discord_ctx.http,
                            "Este comando solo funciona en servidores."
                        )
                        .await;

                    return;
                }
            };

            let voice_channel = match ctx.voice_channel_id {
                Some(id) => id,
                None => {
                    let _ = ctx.channel_id
                        .say(
                            &ctx.discord_ctx.http,
                            "Debes estar en un canal de voz primero."
                        )
                        .await;

                    return;
                }
            };

            if ctx.args.is_empty() {
                let _ = ctx.channel_id
                    .say(
                        &ctx.discord_ctx.http,
                        "Pero dime qué pongo, saranbambich"
                    )
                    .await;

                return;
            }

            // ── Query ──────────────────────────────────────────────────────

            let query = ctx.args.join(" ");

            // ── Songbird ───────────────────────────────────────────────────

            let manager = songbird::get(&ctx.discord_ctx)
                .await
                .expect("Songbird no fue registrado en el cliente");

            let handler = match manager.join(guild_id, voice_channel).await {
                Ok(h) => h,

                Err(e) => {
                    let _ = ctx.channel_id
                        .say(
                            &ctx.discord_ctx.http,
                            format!("Error al conectar: {:?}", e)
                        )
                        .await;

                    return;
                }
            };

            // ── Resolver + Encolar ─────────────────────────────────────────

            match ctx.music_manager
                .resolve_and_enqueue(
                    guild_id,
                    handler,
                    &query,
                    ctx.author_name.clone(),
                    ctx.channel_id,
                    ctx.discord_ctx.http.clone(),
                )
                .await
            {
                Ok(status) => {
                    let embed = match status {
                        PlaybackStatus::PlayingNow(track) => {
                            build_track_embed(TrackEmbedOptions {
                                track:        &track,
                                requested_by: &ctx.author_name,
                                position:     None,
                                color:        COLOR_PLAYING,
                                title_prefix: "🎵 Estás escuchando:",
                            })
                        },
                        PlaybackStatus::Enqueued { track, position } => {
                            build_queue_embed(
                                &track,
                                &ctx.author_name,
                                position,
                            )
                        }
                    };

                    let _ = ctx.channel_id
                        .send_message(&ctx.discord_ctx.http, CreateMessage::new().embed(embed))
                        .await;
                }

                Err(e) => {
                    eprintln!("[ERROR] play: {}", e);

                    let _ = ctx.channel_id
                        .say(
                            &ctx.discord_ctx.http,
                            format!("Error al reproducir: {}", e)
                        )
                        .await;
                }
            }
        })
    }
}