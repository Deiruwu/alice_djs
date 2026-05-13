use std::fs;
use serde::Deserialize;

/// Configuracion global del bot, deserializada desde config.json.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub token:        String,
    pub prefix:       String,
    pub audio:        AudioConfig,
    pub microservice: MicroserviceConfig,
    pub paths:        PathsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AudioConfig {
    /// Volumen inicial, 0.0 a 2.0. 1.0 es el valor normal.
    pub volume: f32,
}

#[derive(Debug, Deserialize)]
pub struct MicroserviceConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct PathsConfig {
    /// Path en el servidor remoto donde vive la musica.
    #[serde(rename = "musicPath")]
    pub music_path: String,

    /// Path local montado via sshfs para desarrollo.
    /// Si esta presente, reemplaza a music_path al resolver file_path de un Track.
    /// Si es null o vacio, se usa music_path sin cambios.
    #[serde(rename = "mountPath")]
    pub mount_path: Option<String>,
}

impl Config {
    /// Lee y deserializa config.json desde el directorio de trabajo.
    pub fn load() -> Self {
        let raw = fs::read_to_string("config.json")
            .expect("No se encontro config.json en el directorio de trabajo");
        serde_json::from_str(&raw)
            .expect("config.json tiene un formato invalido")
    }

    /// Devuelve el path efectivo para un file_path dado.
    ///
    /// Si mountPath esta definido y no esta vacio, sustituye el prefijo
    /// de musicPath por mountPath. Util para desarrollo con sshfs.
    ///
    /// Ejemplo:
    ///   musicPath  = "/home/pi/music_storage"
    ///   mountPath  = "/home/dei/music_storage"
    ///   file_path  = "/home/pi/music_storage/Yz91CpCX84o.m4a"
    ///   resultado  = "/home/dei/music_storage/Yz91CpCX84o.m4a"
    pub fn resolve_path(&self, file_path: &str) -> String {
        match &self.paths.mount_path {
            Some(mount) if !mount.is_empty() => {
                file_path.replacen(&self.paths.music_path, mount, 1)
            }
            _ => file_path.to_string(),
        }
    }
}