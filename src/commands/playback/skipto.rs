// commands/playback/skipto.rs
use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;

pub struct SkipToCommand;

impl AsyncCommand for SkipToCommand {
    fn name(&self) -> &str { "skipto" }
    fn aliases(&self) -> &[&str] { &["st", "saltara"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Salta hasta la posición indicada de la cola." }
    fn usage(&self) -> &str { "skipto <posición>" }

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

            let position: usize = match ctx.args.first().and_then(|a| a.parse().ok()) {
                Some(n) if n >= 1 => n,
                _ => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Usa: `skipto <número>` (ej: `skipto 3`)").await;
                    return;
                }
            };

            // position es 1-based para el usuario, lo convertimos a 0-based internamente
            match ctx.music_manager.skip_to(guild_id, position - 1).await {
                Ok(track) => {
                    let artists = track.track.artists.iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" & ");
                    let _ = ctx.channel_id.say(
                        &ctx.discord_ctx.http,
                        format!("Saltando a la posición {}: **{} - {}**", position, track.track.title, artists),
                    ).await;
                }
                Err(e) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, format!("Error: {}", e)).await;
                }
            }
        })
    }
}