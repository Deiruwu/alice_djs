use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;

pub struct SkipCommand;

impl Command for SkipCommand {
    fn name(&self) -> &str { "skip" }
    fn aliases(&self) -> &[&str] { &["s", "saltear"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Salta la canción actual." }
    fn usage(&self) -> &str { "skip" }

    fn execute<'a>(&'a self, ctx: &'a CommandContext) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = ctx.guild_id.unwrap();

            match ctx.music_manager.skip(guild_id).await {
                Ok(_) => {
                    ctx.reply("Saltando canción...").await;
                }
                Err(e) => {
                    eprintln!("[Error] skip: {}", e);
                }
            }
        })
    }
}