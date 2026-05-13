use serenity::{model::channel::Message, prelude::*};
use songbird::SerenityInit; // re-exportado; solo necesario si se usa fuera de main

// ---------------------------------------------------------------------------
// Comandos que interactuan con canales de voz a traves de songbird.
//
// Requisitos previos:
//   1. .register_songbird() en el Client builder de main.rs
//   2. Intent GUILD_VOICE_STATES activo
//   3. El usuario que invoca debe estar en un canal de voz del mismo servidor
// ---------------------------------------------------------------------------

/// Une al bot al canal de voz donde se encuentra el autor del mensaje.
///
/// Flujo:
///   1. Obtener GuildId del mensaje (falla silenciosamente en DMs).
///   2. Buscar el VoiceState del autor en la cache del servidor.
///   3. Recuperar el SongbirdManager del TypeMap del cliente.
///   4. Llamar a manager.join(guild_id, channel_id) para entrar al canal.
///
/// El objeto `Call` retornado por join() es un Arc<Mutex<Call>> que permite:
///   - call.lock().await.play_source(...)  -> reproducir audio (requiere fuente)
///   - call.lock().await.mute(true)        -> silenciar al bot
///   - call.lock().await.deafen(true)      -> ensordecer al bot
///   - call.lock().await.add_global_event  -> escuchar eventos de voz (habla, silencios)
///
/// OPCIONAL: guardar el Arc<Mutex<Call>> en el TypeMap si otros modulos necesitan
///           acceder al canal de voz sin pasar por songbird::get().
pub async fn join(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            let _ = msg.channel_id.say(&ctx.http, "Este comando solo funciona en un servidor.").await;
            return;
        }
    };

    // Buscar en que canal de voz esta el usuario que invoca el comando.
    // guild.voice_states es un HashMap<UserId, VoiceState> disponible con la cache.
    // OPCIONAL: si no usas cache, obtener el canal via HTTP con guild_id.to_partial_guild().
    let channel_id = {
        let guild = match ctx.cache.guild(guild_id) {
            Some(g) => g,
            None => {
                let _ = msg.channel_id.say(&ctx.http, "No se pudo obtener el servidor de la cache.").await;
                return;
            }
        };
        match guild.voice_states.get(&msg.author.id).and_then(|vs| vs.channel_id) {
            Some(ch) => ch,
            None => {
                let _ = msg.channel_id.say(&ctx.http, "Debes estar en un canal de voz para usar este comando.").await;
                return;
            }
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird no fue registrado en el cliente");

    match manager.join(guild_id, channel_id).await {
        Ok(_call) => {
            // _call es Arc<Mutex<Call>>; guardarlo si se necesita reproducir audio despues.
            // OPCIONAL: _call.lock().await.deafen(true).await para unirse en modo escucha.
            let _ = msg.channel_id.say(&ctx.http, "Conectado al canal de voz.").await;
        }
        Err(e) => {
            eprintln!("[join] Error al unirse al canal: {:?}", e);
            let _ = msg.channel_id.say(&ctx.http, "No se pudo conectar al canal de voz.").await;
        }
    }
}

/// Desconecta al bot del canal de voz en el servidor actual.
///
/// Si el bot no esta en ningun canal de voz del servidor, responde con aviso.
/// OPCIONAL: detener cualquier reproduccion activa antes de salir con
///   call.lock().await.stop() si se esta usando audio.
pub async fn leave(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            let _ = msg.channel_id.say(&ctx.http, "Este comando solo funciona en un servidor.").await;
            return;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird no fue registrado en el cliente");

    if manager.get(guild_id).is_none() {
        let _ = msg.channel_id.say(&ctx.http, "No estoy en ningun canal de voz.").await;
        return;
    }

    match manager.remove(guild_id).await {
        Ok(_) => {
            let _ = msg.channel_id.say(&ctx.http, "Desconectado del canal de voz.").await;
        }
        Err(e) => {
            eprintln!("[leave] Error al salir del canal: {:?}", e);
            let _ = msg.channel_id.say(&ctx.http, "No se pudo desconectar del canal de voz.").await;
        }
    }
}