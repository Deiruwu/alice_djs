use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;

pub struct ClearCommand;

impl Command for ClearCommand {
    fn name(&self) -> &str { "clear" }
    fn aliases(&self) -> &[&str] { &["cl"] }
    fn category(&self) -> &str { "Playback" }
    fn description(&self) -> &str { "Limpia toda la cola de reproducción." }
    fn usage(&self) -> &str { "clear" }

    fn execute<'a>(&'a self, ctx: &'a CommandContext) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = ctx.guild_id.unwrap();

            match ctx.music_manager.clear_queue(guild_id).await {
                Ok(_) => {
                    ctx.reply("Cola limpiada").await;
                }
                Err(e) => {
                    eprintln!("[ERROR] clear: {}", e);
                }
            }
        })
    }
}