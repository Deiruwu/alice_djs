use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;

pub struct PlayCommand;

impl AsyncCommand for PlayCommand {
    fn name(&self) -> &str { "play" }

    fn aliases(&self) -> &[&str] { &["p"] }

    fn category(&self) -> &str { "Música" }

    fn description(&self) -> &str { "Busca y reproduce una cancion." }

    fn usage(&self) -> &str { "play <busqueda o URL>" }

    fn execute_async<'a>(
        &'a self,
        ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = match ctx.guild_id {
                Some(id) => id,
                None => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Este comando solo funciona en servidores.").await;
                    return;
                }
            };

            let voice_channel = match ctx.voice_channel_id {
                Some(id) => id,
                None => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Debes estar en un canal de voz primero.").await;
                    return;
                }
            };

            if ctx.args.is_empty() {
                let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Especificame que poner").await;
                return;
            }
            let query = ctx.args.join(" ");

            let manager = songbird::get(&ctx.discord_ctx)
                .await
                .expect("Songbird no fue registrado en el cliente");

            let handler = match manager.join(guild_id, voice_channel).await {
                Ok(h) => h,
                Err(e) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, format!("Error al conectar: {:?}", e)).await;
                    return;
                }
            };

            match ctx.music_manager.resolve_and_enqueue(guild_id, handler, &query, ctx.author_name.clone()).await {
                Ok(track_title) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, format!("Añadido a la cola: **{}**", track_title)).await;
                }
                Err(e) => {
                    let _ = println!("[ERROR] Error al reproducir: {}", e);
                }
            }
        })
    }
}