// commands/playback/queue.rs
use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::AsyncCommand;

pub struct QueueCommand;

impl AsyncCommand for QueueCommand {
    fn name(&self) -> &str { "queue" }
    fn aliases(&self) -> &[&str] { &["q", "cola"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Muestra la cola de reproducción." }
    fn usage(&self) -> &str { "queue" }

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

            let queue_info = ctx.music_manager.get_queue(guild_id).await;

            match queue_info {
                None => {
                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, "No hay nada en cola.").await;
                }
                Some((current, queue)) => {
                    let mut msg = String::new();

                    if let Some(current_track) = current {
                        let artists = current_track.track.artists.iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" & ");
                        msg.push_str(&format!(
                            "**Reproduciendo ahora:**\n`{}` — {} • Pedido por: {}\n\n",
                            current_track.track.title, artists, current_track.requested_by
                        ));
                    }

                    if queue.is_empty() {
                        msg.push_str("No hay más canciones en cola.");
                    } else {
                        msg.push_str("**Cola:**\n");
                        for (i, queued) in queue.iter().enumerate() {
                            let artists = queued.track.artists.iter()
                                .map(|a| a.name.as_str())
                                .collect::<Vec<_>>()
                                .join(" & ");
                            msg.push_str(&format!(
                                "`{}.` {} — {} • Pedido por: {}\n",
                                i + 1,
                                queued.track.title,
                                artists,
                                queued.requested_by
                            ));
                        }
                    }

                    let _ = ctx.channel_id.say(&ctx.discord_ctx.http, msg).await;
                }
            }
        })
    }
}