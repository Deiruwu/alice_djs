use std::future::Future;
use std::pin::Pin;
use crate::commands::command_context::CommandContext;
use crate::commands::traits::{AsyncCommand, Command};

// ---------------------------------------------------------------------------
// Ping - comando sincrono de ejemplo
// ---------------------------------------------------------------------------

pub struct PingCommand;

impl Command for PingCommand {
    fn name(&self) -> &str { "ping" }
    fn aliases(&self) -> &[&str] { &["p"] }
    fn category(&self) -> &str { "General" }
    fn description(&self) -> &str { "Comprueba que el bot esta vivo." }
    fn usage(&self) -> &str { "ping" }

    fn execute(&self, _ctx: &CommandContext) {
        // Por ahora no hace nada. Reply se implementara despues.
        println!("[PingCommand] execute() llamado.");
    }
}

// ---------------------------------------------------------------------------
// SlowPing - comando asincrono de ejemplo
// ---------------------------------------------------------------------------

pub struct SlowPingCommand;

impl AsyncCommand for SlowPingCommand {
    fn name(&self) -> &str { "slowping" }
    fn aliases(&self) -> &[&str] { &["sp"] }
    fn category(&self) -> &str { "General" }
    fn description(&self) -> &str { "Ping pero asincrono, para probar el flujo async." }
    fn usage(&self) -> &str { "slowping" }

    fn execute_async<'a>(
        &'a self,
        _ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Por ahora no hace nada. Reply se implementara despues.
            println!("[SlowPingCommand] execute_async() llamado.");
        })
    }
}