use std::collections::HashMap;
use super::traits::{AsyncCommand, Command};
use super::command_context::CommandContext;

/// Almacena y despacha comandos sincronos y asincronos.
///
/// Los comandos se indexan por nombre y por cada alias,
/// apuntando todos al mismo indice en la lista maestra.
/// La lista maestra mantiene el orden de registro y evita duplicados.
pub struct CommandRegistry {
    /// Mapa de nombre/alias -> indice en sync_commands.
    sync_index: HashMap<String, usize>,

    /// Lista maestra de comandos sincronos en orden de registro.
    sync_commands: Vec<Box<dyn Command>>,

    /// Mapa de nombre/alias -> indice en async_commands.
    async_index: HashMap<String, usize>,

    /// Lista maestra de comandos asincronos en orden de registro.
    async_commands: Vec<Box<dyn AsyncCommand>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            sync_index:     HashMap::new(),
            sync_commands:  Vec::new(),
            async_index:    HashMap::new(),
            async_commands: Vec::new(),
        }
    }

    /// Registra un comando sincrono y todos sus alias.
    pub fn register(&mut self, cmd: Box<dyn Command>) {
        let idx = self.sync_commands.len();
        self.sync_index.insert(cmd.name().to_lowercase(), idx);
        for alias in cmd.aliases() {
            self.sync_index.insert(alias.to_lowercase(), idx);
        }
        self.sync_commands.push(cmd);
    }

    /// Registra un comando asincrono y todos sus alias.
    pub fn register_async(&mut self, cmd: Box<dyn AsyncCommand>) {
        let idx = self.async_commands.len();
        self.async_index.insert(cmd.name().to_lowercase(), idx);
        for alias in cmd.aliases() {
            self.async_index.insert(alias.to_lowercase(), idx);
        }
        self.async_commands.push(cmd);
    }

    /// Busca y ejecuta un comando por nombre o alias.
    ///
    /// Busca primero en sincronos, luego en asincronos.
    /// Printea el contexto completo en terminal para verificar que llega bien.
    /// Si el label no existe en ninguno de los dos, lo indica sin explotar.
    pub async fn dispatch(&self, label: &str, ctx: &CommandContext) {
        let key = label.to_lowercase();

        if let Some(&idx) = self.sync_index.get(&key) {
            let cmd = &self.sync_commands[idx];
            println!("[dispatch] '{}' | {:#?}", cmd.name(), ctx);
            cmd.execute(ctx);
            return;
        }

        if let Some(&idx) = self.async_index.get(&key) {
            let cmd = &self.async_commands[idx];
            println!("[dispatch_async] '{}' | {:#?}", cmd.name(), ctx);
            cmd.execute_async(ctx).await;
            return;
        }

        println!("[dispatch] Comando desconocido: '{}'", label);
    }
}