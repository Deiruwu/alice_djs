mod audio;
mod bot;
mod commands;
mod config;
mod microservices;
mod model;

use std::sync::Arc;
use serenity::prelude::*;
use songbird::SerenityInit;

use audio::MusicManager;
use bot::Bot;
use commands::CommandRegistry;
use config::Config;
use commands::playback::*;

#[tokio::main]
async fn main() {
    let config = Arc::new(Config::load());

    println!("[config] Prefix: {}", config.prefix);
    println!("[config] Microservice: {}:{}", config.microservice.host, config.microservice.port);
    println!("[config] Music path efectivo: {}",
             config.resolve_path(&config.paths.music_path));

    let mut registry = CommandRegistry::new();
    registry.register(Box::new(PlayCommand));
    registry.register(Box::new(PlayNextCommand));
    registry.register(Box::new(ClearCommand));
    registry.register(Box::new(QueueCommand));
    registry.register(Box::new(SkipCommand));
    registry.register(Box::new(SkipToCommand));
    registry.register(Box::new(VolumeCommand));
    registry.register(Box::new(RadioCommand));

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    // ─── INSTANCIACIÓN PREVIA ────────────────────────────────────────────────
    // Creamos el MusicManager antes para poder compartirlo con el hilo de apagado
    let music_manager = Arc::new(MusicManager::new(Arc::clone(&config)));

    // Clonamos el Arc para el Bot y otro para el Shutdown
    let music_manager_for_bot = Arc::clone(&music_manager);
    let music_manager_for_shutdown = Arc::clone(&music_manager);

    let mut client = Client::builder(&config.token, intents)
        .event_handler(Bot {
            registry:      Arc::new(registry),
            music_manager: music_manager_for_bot,
        })
        .register_songbird()
        .await
        .expect("Error al crear el cliente");

    // ─── GRACEFUL SHUTDOWN (Ctrl+C) ──────────────────────────────────────────

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Error al registrar el manejador Ctrl-C");
        println!("\n[SHUTDOWN] Señal Ctrl+C recibida. Limpiando procesos de audio...");

        // Usamos nuestro propio manager para destruir los procesos
        music_manager_for_shutdown.shutdown_all().await;

        println!("[SHUTDOWN] Apagando conexión a Discord...");
        shard_manager.shutdown_all().await;
    });

    // ─── INICIO DEL BOT ──────────────────────────────────────────────────────

    if let Err(e) = client.start().await {
        eprintln!("[bot] Error fatal: {:?}", e);
    }
}