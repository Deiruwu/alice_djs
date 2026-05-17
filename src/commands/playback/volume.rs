use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::command_trait::Command;

pub struct VolumeCommand;

impl Command for VolumeCommand {
    fn name(&self) -> &str { "volume" }
    fn aliases(&self) -> &[&str] { &["vol", "v"] }
    fn category(&self) -> &str { "Música" }
    fn description(&self) -> &str { "Muestra o ajusta el volumen (0–100)." }
    fn usage(&self) -> &str { "volume [0-100]" } // Actualizado para indicar que es opcional

    // Por defecto min_args es 0, lo cual es perfecto aquí porque permite que pase sin argumentos.

    fn execute<'a>(&'a self, ctx: &'a CommandContext) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let guild_id = ctx.guild_id.unwrap();

            // 1. Rama sin argumentos: Mostrar volumen actual
            if ctx.args.is_empty() {
                match ctx.music_manager.get_volume(guild_id).await {
                    Some(vol) => {
                        let emoji = if vol == 0 { "🔇" } else if vol < 40 { "🔈" } else if vol < 75 { "🔉" } else { "🔊" };
                        ctx.reply(format!("{} El volumen actual es **{}%**", emoji, vol)).await;
                    }
                    None => {
                        ctx.reply("No hay nada reproduciéndose o no pude obtener el volumen.").await;
                    }
                }
                return;
            }

            let level: u8 = match ctx.args.first().and_then(|a| a.parse().ok()) {
                Some(n) if n <= 100 => n,
                _ => {
                    ctx.reply_usage(self.usage()).await;
                    return;
                }
            };

            match ctx.music_manager.set_volume(guild_id, level).await {
                Ok(_) => {
                    let emoji = if level == 0 { "🔇" } else if level < 40 { "🔈" } else if level < 75 { "🔉" } else { "🔊" };
                    ctx.reply(format!("{} Volumen ajustado a **{}%**", emoji, level)).await;
                }
                Err(e) => {
                    eprintln!("[Error] volume: {}", e);
                }
            }
        })
    }
}