use std::future::Future;
use std::pin::Pin;

use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;

use crate::bot::{build_radio_embed, RadioEmbedOptions};

pub struct RadioCommand;

impl Command for RadioCommand {
    fn name(&self) -> &str { "radio" }
    fn aliases(&self) -> &[&str] { &["mix", "r"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Inicia una radio automática basada en una canción o artista." }
    fn usage(&self) -> &str { "radio <búsqueda o URL>" }
    fn min_args(&self) -> usize { 1 }

    fn execute<'a>(
        &'a self,
        ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = ctx.guild_id.unwrap();
            let voice_channel = ctx.voice_channel_id.unwrap();
            let query = ctx.args.join(" ");

            let manager = songbird::get(&ctx.discord_ctx)
                .await
                .expect("Songbird no fue registrado");

            let handler = match manager.join(guild_id, voice_channel).await {
                Ok(h) => h,
                Err(e) => {
                    ctx.reply(format!("Error al conectar: {:?}", e)).await;
                    return;
                }
            };

            // Notificamos que estamos pensando
            ctx.reply("📻 `Buscando semilla y generando estación...`").await;

            match ctx.music_manager
                .resolve_and_enqueue_radio(
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
                Ok((enqueued_count, seed_track)) => {
                    let author_nick = ctx.author_nick.as_deref().unwrap_or(&ctx.author_name);

                    let embed = build_radio_embed(RadioEmbedOptions {
                        seed_track:      &seed_track,
                        enqueued_count,
                        requested_by:    author_nick,
                        author_icon_url: ctx.author_avatar.as_deref(),
                    });

                    ctx.reply_embed(embed).await;
                }
                Err(e) => {
                    eprintln!("[ERROR] radio: {}", e);
                }
            }
        })
    }
}