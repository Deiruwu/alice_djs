use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::net::TcpStream;
use serde::Deserialize;
use serde_json::json;
use crate::config::Config;
use crate::model::Track;


/// Errores posibles al hablar con el microservicio.
#[derive(Debug)]
pub enum MicroserviceError {
    /// No se pudo establecer la conexion TCP.
    ConnectionFailed(std::io::Error),
    /// Error al enviar o recibir datos.
    IoError(std::io::Error),
    /// El microservicio respondio con status != "ok".
    ServiceError(String),
    /// La respuesta no era JSON valido.
    InvalidResponse(String),
}

impl std::fmt::Display for MicroserviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(e) => write!(f, "Conexion fallida: {}", e),
            Self::IoError(e)          => write!(f, "Error de IO: {}", e),
            Self::ServiceError(s)     => write!(f, "Error del servicio: {}", s),
            Self::InvalidResponse(s)  => write!(f, "Respuesta invalida: {}", s),
        }
    }
}

impl std::error::Error for MicroserviceError {}

/// Estructura transitoria para deserializar la respuesta del microservicio Python
#[derive(Deserialize)]
struct ResolveResponse {
    status: String,
    data: Option<Track>,
    message: Option<String>,
}

/// Cliente TCP para el microservicio de musica.
pub struct MicroserviceClient {
    addr: String,
}

impl MicroserviceClient {
    pub fn new(config: &Arc<Config>) -> Self {
        Self {
            addr: format!("{}:{}", config.microservice.host, config.microservice.port),
        }
    }

    /// Envia una accion `resolve` con la query dada y devuelve un Vec<Track>.
    pub async fn resolve(&self, query: &str) -> Result<Track, MicroserviceError> {
        let payload = json!({
            "action": "resolve",
            "query": query
        }).to_string() + "\n";

        let raw = self.send_raw(&payload).await?;

        let response: ResolveResponse = serde_json::from_str(&raw)
            .map_err(|e| MicroserviceError::InvalidResponse(format!("Fallo parseo: {}. Raw: {}", e, raw)))?;

        if response.status == "ok" {
            // Extraemos el Track, si es null tiramos error lógico
            response.data.ok_or_else(|| MicroserviceError::ServiceError("El microservicio devolvió ok pero 'data' es null".into()))
        } else {
            Err(MicroserviceError::ServiceError(
                response.message.unwrap_or_else(|| "Error desconocido del microservicio".into())
            ))
        }
    }

    /// Abre conexion, envia payload, lee UNA SOLA LINEA de respuesta.
    async fn send_raw(&self, payload: &str) -> Result<String, MicroserviceError> {
        let mut stream = TcpStream::connect(&self.addr)
            .await
            .map_err(MicroserviceError::ConnectionFailed)?;

        stream.write_all(payload.as_bytes())
            .await
            .map_err(MicroserviceError::IoError)?;

        // Envolvemos el stream en un BufReader para poder leer línea por línea
        let mut reader = BufReader::new(stream);
        let mut response = String::new();

        // Lee hasta encontrar un '\n'. No espera a que Python cierre la conexión.
        reader.read_line(&mut response)
            .await
            .map_err(MicroserviceError::IoError)?;

        // Al terminar el scope de esta función, el `stream` hace drop y
        // Rust cierra la conexión automáticamente (equivalente a socket.destroy() en JS).
        Ok(response.trim().to_string())
    }
}