use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;

pub struct ClearCommand;

impl AsyncCommand for ClearCommand {
    fn name(&self) -> &str { "clear" }
    fn aliases(&self) -> &[&str] { &["limpar", "limpiar"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Limpia toda la cola de reproducción." }
    fn usage(&self) -> &str { "clear" }

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

            match ctx.music_manager.clear_queue(guild_id).await {
                Ok(_) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "Cola limpiada.").await;
                }
                Err(e) => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, format!("Error: {}", e)).await;
                }
            }
        })
    }
}