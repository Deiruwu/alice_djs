// commands/playback/volume.rs
use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;

pub struct VolumeCommand;

impl AsyncCommand for VolumeCommand {
    fn name(&self) -> &str { "volume" }
    fn aliases(&self) -> &[&str] { &["vol", "v"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Ajusta el volumen (0–100)." }
    fn usage(&self) -> &str { "volume <0-100>" }

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

            let level: u8 = match ctx.args.first().and_then(|a| a.parse().ok()) {
                Some(n) if n <= 100 => n,
                _ => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Usa: `volume <0-100>` (ej: `volume 75`)").await;
                    return;
                }
            };

            match ctx.music_manager.set_volume(guild_id, level).await {
                Ok(_) => {
                    let emoji = if level == 0 { "🔇" } else if level < 40 { "🔈" } else if level < 75 { "🔉" } else { "🔊" };
                    let _ = ctx.channel_id.say(
                        &ctx.discord_ctx.http,
                        format!("{} Volumen ajustado a **{}%**", emoji, level),
                    ).await;
                }
                Err(e) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, format!("Error: {}", e)).await;
                }
            }
        })
    }
}