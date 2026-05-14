use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;

pub struct SkipCommand;

impl AsyncCommand for SkipCommand {
    fn name(&self) -> &str { "skip" }
    fn aliases(&self) -> &[&str] { &["s", "saltear"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Salta la canción actual." }
    fn usage(&self) -> &str { "skip" }

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

            match ctx.music_manager.skip(guild_id).await {
                Ok(_) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Saltando canción...").await;
                }
                Err(e) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, format!("Error: {}", e)).await;
                }
            }
        })
    }
}