use std::future::Future;
use std::pin::Pin;
use super::command_context::CommandContext;

pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] { &[] }
    fn category(&self) -> &str;
    fn description(&self) -> &str;
    fn usage(&self) -> &str;

    // --- Validaciones ---
    fn guild_only(&self) -> bool { true }
    fn voice_required(&self) -> bool { true }
    fn min_args(&self) -> usize { 0 }

    // --------------------

    fn execute<'a>(
        &'a self,
        ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}