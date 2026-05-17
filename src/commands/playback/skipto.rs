use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;

pub struct SkipToCommand;

impl Command for SkipToCommand {
    fn name(&self) -> &str { "skipto" }
    fn aliases(&self) -> &[&str] { &["st", "saltara"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Salta hasta la posición indicada de la cola." }
    fn usage(&self) -> &str { "skipto <posición>" }
    fn min_args(&self) -> usize { 1 }

    fn execute<'a>(&'a self, ctx: &'a CommandContext) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = ctx.guild_id.unwrap();

            let position: usize = match ctx.args.first().and_then(|a| a.parse().ok()) {
                Some(n) if n >= 1 => n,
                _ => {
                    ctx.reply_usage(self.usage()).await;
                    return;
                }
            };

            match ctx.music_manager.skip_to(guild_id, position - 1).await {
                Ok(track) => {
                    let artists = track.track.artists.iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" & ");

                    ctx.reply(format!("Saltando a la posición {}: **{} - {}**", position, track.track.title, artists)).await;
                }
                Err(e) => {
                    eprintln!("[Error] skipto: {}", e);
                }
            }
        })
    }
}