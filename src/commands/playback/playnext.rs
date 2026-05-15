use std::future::Future;
use std::pin::Pin;

use serenity::builder::CreateMessage;

use crate::bot::*;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;
use crate::audio::PlaybackStatus;

pub struct PlayNextCommand;

impl AsyncCommand for PlayNextCommand {

    fn name(&self) -> &str { "playnext" }

    fn aliases(&self) -> &[&str] {
        &["pn", "siguiente"]
    }

    fn category(&self) -> &str {
        "Música"
    }

    fn description(&self) -> &str {
        "Añade una canción al principio de la cola (prioridad)."
    }

    fn usage(&self) -> &str {
        "playnext <búsqueda o URL>"
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
                        "Dime qué canción poner primero."
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
                .resolve_and_enqueue_next(
                    guild_id,
                    handler,
                    &query,
                    ctx.author_name.clone(),
                    ctx.author_avatar.clone(),
                    ctx.channel_id,
                    ctx.voice_channel_id.unwrap(),
                    ctx.discord_ctx.http.clone(),
                )
                .await
            {
                Ok(status) => {
                    // ── Embed ──────────────────────────────────────────────

                    let embed = match status {
                        PlaybackStatus::PlayingNow(track) => {
                            build_track_embed(TrackEmbedOptions {
                                track:        &track,
                                requested_by: &ctx.author_nick.as_ref().unwrap_or(&ctx.author_name),
                                author_icon_url: ctx.author_avatar.as_deref(),
                                position:     None,
                                color:        COLOR_PLAYING,
                                title_prefix: "🎵 Reproduciendo",
                            })
                        },
                        PlaybackStatus::Enqueued { track, position } => {
                            build_queue_embed(
                                &track,
                                &ctx.author_nick.as_ref().unwrap_or(&ctx.author_name),
                                ctx.author_avatar.as_deref(),
                                position,
                            )
                        }
                    };

                    // ── Enviar mensaje ───────────────────────────────────

                    let _ = ctx.channel_id
                        .send_message(
                            &ctx.discord_ctx.http,
                            CreateMessage::new().embed(embed),
                        )
                        .await;
                }

                Err(e) => {
                    eprintln!("[ERROR] playnext: {}", e);

                    let _ = ctx.channel_id
                        .say(
                            &ctx.discord_ctx.http,
                            format!("Error: {}", e)
                        )
                        .await;
                }
            }
        })
    }
}