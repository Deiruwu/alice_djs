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
    registry.register_async(Box::new(PlayCommand));
    registry.register_async(Box::new(PlayCommand));
    registry.register_async(Box::new(PlayNextCommand));
    registry.register_async(Box::new(ClearCommand));
    registry.register_async(Box::new(QueueCommand));
    registry.register_async(Box::new(SkipCommand));
    registry.register_async(Box::new(SkipToCommand));
    registry.register_async(Box::new(VolumeCommand));

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&config.token, intents)
        .event_handler(Bot {
            registry:      Arc::new(registry),
            music_manager: Arc::new(MusicManager::new(Arc::clone(&config))),
        })
        .register_songbird()
        .await
        .expect("Error al crear el cliente");

    if let Err(e) = client.start().await {
        eprintln!("[bot] Error fatal: {:?}", e);
    }
}