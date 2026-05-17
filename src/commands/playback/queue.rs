use std::future::Future;
use std::pin::Pin;

use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;

use crate::bot::build_full_queue_embed;

pub struct QueueCommand;

impl Command for QueueCommand {
    fn name(&self) -> &str { "queue" }
    fn aliases(&self) -> &[&str] { &["q", "cola"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Muestra la cola de reproducción." }
    fn usage(&self) -> &str { "queue" }
    fn voice_required(&self) -> bool { false }

    fn execute<'a>(
        &'a self,
        ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = ctx.guild_id.unwrap();

            let queue_info = ctx.music_manager.get_queue(guild_id).await;

            match queue_info {
                None => {
                    ctx.reply("El reproductor no está activo en este servidor.").await;
                }
                Some((current, queue)) => {
                    if current.is_none() && queue.is_empty() {
                        ctx.reply("No hay nada en reproducción ni en cola.").await;
                        return;
                    }

                    let embed = build_full_queue_embed(current.as_ref(), &queue, 10);

                    ctx.reply_embed(embed).await;
                }
            }
        })
    }
}