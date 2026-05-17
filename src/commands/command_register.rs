use std::collections::HashMap;
use super::command_trait::Command;
use super::command_context::CommandContext;

pub struct CommandRegistry {
    index: HashMap<String, usize>,
    commands: Vec<Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            commands: Vec::new(),
        }
    }

    pub fn register(&mut self, cmd: Box<dyn Command>) {
        let idx = self.commands.len();

        self.index.insert(cmd.name().to_lowercase(), idx);
        for alias in cmd.aliases() {
            self.index.insert(alias.to_lowercase(), idx);
        }

        self.commands.push(cmd);
    }

    pub async fn dispatch(&self, label: &str, ctx: &CommandContext) {
        let key = label.to_lowercase();

        if let Some(&idx) = self.index.get(&key) {
            let cmd = &self.commands[idx];

            if cmd.guild_only() && ctx.guild_id.is_none() {
                ctx.reply("Este comando solo funciona en servidores.").await;
                return;
            }

            if cmd.voice_required() && ctx.voice_channel_id.is_none() {
                ctx.reply("Debes estar en un canal de voz para usar esto.").await;
                return;
            }

            if ctx.args.len() < cmd.min_args() {
                ctx.reply_usage(cmd.usage()).await;
                return;
            }

            // ── EJECUCIÓN ──────────────────────────────────────────────

            println!("[dispatch] Ejecutando '{}' | {:#?}", cmd.name(), ctx);
            cmd.execute(ctx).await;
            return;
        }

        println!("[dispatch] Comando desconocido: '{}'", label);
    }
}