use std::future::Future;
use std::pin::Pin;
use super::command_context::CommandContext;

/// Contrato base que todo comando debe cumplir.
///
/// Equivale a la clase abstracta Command de Java, pero como trait puro.
/// Ningun metodo conoce serenity; el unico punto de contacto con Discord
/// es CommandContext, que llega ya construido.
pub trait Command: Send + Sync {
    /// Nombre principal del comando, en minusculas. Ej: "ping"
    fn name(&self) -> &str;

    /// Lista de alias con los que tambien se puede invocar. Puede ser vacia.
    fn aliases(&self) -> &[&str];

    /// Categoria para agrupar en listados. Ej: "General", "Admin", "Voz"
    fn category(&self) -> &str;

    /// Descripcion corta de lo que hace el comando.
    fn description(&self) -> &str;

    /// Formato de uso. Ej: "ban <usuario> [duracion]"
    fn usage(&self) -> &str;

    /// Logica principal del comando.
    ///
    /// No envia nada por si mismo; lo que necesite comunicar al exterior
    /// lo hara a traves del sistema de respuesta cuando este implementado.
    fn execute(&self, ctx: &CommandContext);
}

/// Contrato para comandos que necesitan ejecutarse de forma asincrona.
///
/// El dispatcher llamara execute_async en lugar de execute.
/// Equivale a AsyncCommand de Java pero sin Thread manual;
/// aqui se usa async/await nativo de Rust.
///
/// La firma con Pin<Box<dyn Future>> es la forma idiomatica de declarar
/// un metodo async dentro de un trait en Rust estable sin dependencias extra.
pub trait AsyncCommand: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str];
    fn category(&self) -> &str;
    fn description(&self) -> &str;
    fn usage(&self) -> &str;

    /// Version asincrona de execute.
    ///
    /// FUTURO: aqui iran operaciones de red, consultas a base de datos,
    ///         llamadas a APIs externas, etc.
    fn execute_async<'a>(
        &'a self,
        ctx: &'a CommandContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}