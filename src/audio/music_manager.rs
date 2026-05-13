use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serenity::model::id::GuildId;
use serenity::async_trait;
use crate::config::Config;
use crate::microservices::MicroserviceClient;
use super::track_scheduler::TrackScheduler;
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};

#[derive(Debug)]
pub enum MusicError {
    ResolveError(String),
    NoFilePath,
    NoScheduler,
}

impl std::fmt::Display for MusicError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ResolveError(s) => write!(f, "Error resolviendo: {}", s),
            Self::NoFilePath      => write!(f, "La cancion no tiene file_path"),
            Self::NoScheduler     => write!(f, "No hay sesion activa en este servidor"),
        }
    }
}

impl std::error::Error for MusicError {}

pub struct MusicManager {
    config:     Arc<Config>,
    client:     MicroserviceClient,
    schedulers: Mutex<HashMap<GuildId, Mutex<TrackScheduler>>>,
}

impl MusicManager {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            client:     MicroserviceClient::new(&config),
            config,
            schedulers: Mutex::new(HashMap::new()),
        }
    }

    pub async fn resolve_and_enqueue(
        &self,
        guild_id: GuildId,
        handler: Arc<Mutex<Call>>,
        query:    &str,
        requested_by: String,
    ) -> Result<String, MusicError> {
        let mut track = self.client
            .resolve(query)
            .await
            .map_err(|e| MusicError::ResolveError(e.to_string()))?;


        let track_title = track.title.clone();

        let file_path_raw = track.file_path.clone()
            .ok_or(MusicError::NoFilePath)?;

        let path_efectivo = self.config.resolve_path(&file_path_raw);

        // --- TELEMETRIA DE I/O ---
        println!("[I/O Debug] Intentando leer archivo en: {}", path_efectivo);
        match std::fs::metadata(&path_efectivo) {
            Ok(meta) => println!("[I/O Debug] Archivo existe. Tamaño: {} bytes. Permisos: {:?}", meta.len(), meta.permissions()),
            Err(e) => eprintln!("[I/O Debug] Falla del sistema de archivos: {}", e),
        }

        let input = crate::audio::create_input(&path_efectivo)
            .map_err(|e| MusicError::ResolveError(format!("Error de encoder: {}", e)))?;

        self.get_or_create_scheduler(guild_id)
            .await
            .lock()
            .await
            .enqueue(track, requested_by);

        let mut call = handler.lock().await;

        let track_handle = call.play_input(input);
        track_handle.add_event(Event::Track(songbird::TrackEvent::Error), AudioErrorHandler).unwrap();

        Ok(track_title)
    }

    pub async fn queue_len(&self, guild_id: GuildId) -> usize {
        let schedulers = self.schedulers.lock().await;
        match schedulers.get(&guild_id) {
            Some(s) => s.lock().await.len(),
            None    => 0,
        }
    }

    pub async fn queue_list(&self, guild_id: GuildId) -> Vec<String> {
        let schedulers = self.schedulers.lock().await;
        match schedulers.get(&guild_id) {
            Some(s) => s.lock().await
                .list()
                .map(|qt| qt.track.title.clone())
                .collect(),
            None => vec![],
        }
    }

    pub async fn skip(&self, guild_id: GuildId) -> Result<(), MusicError> {
        let schedulers = self.schedulers.lock().await;
        let scheduler = schedulers
            .get(&guild_id)
            .ok_or(MusicError::NoScheduler)?;
        scheduler.lock().await.next();
        Ok(())
    }

    pub async fn clear(&self, guild_id: GuildId) -> Result<(), MusicError> {
        let schedulers = self.schedulers.lock().await;
        let scheduler = schedulers
            .get(&guild_id)
            .ok_or(MusicError::NoScheduler)?;
        scheduler.lock().await.clear();
        Ok(())
    }

    async fn get_or_create_scheduler(
        &self,
        guild_id: GuildId,
    ) -> &Mutex<TrackScheduler> {
        let mut schedulers = self.schedulers.lock().await;
        schedulers
            .entry(guild_id)
            .or_insert_with(|| Mutex::new(TrackScheduler::new()));

        unsafe {
            let ptr = schedulers.get(&guild_id).unwrap() as *const Mutex<TrackScheduler>;
            &*ptr
        }
    }
}

pub struct AudioErrorHandler;

#[async_trait]
impl VoiceEventHandler for AudioErrorHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, _) in *track_list {
                eprintln!("[Driver de Audio] CRASH CRITICO en la pista:\n{:#?}", state.playing);
            }
        }
        None
    }
}