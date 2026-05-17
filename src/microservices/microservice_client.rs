use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::net::TcpStream;
use serde::Deserialize;
use serde_json::json;
use crate::config::Config;
use crate::model::Track;
use crate::microservices::errors::MicroserviceError;


/// Estructura transitoria para deserializar la respuesta del microservicio Python
#[derive(Deserialize)]
struct ApiResponse<T> {
    status: String,
    data: Option<T>,
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

    /// Envia una acción `resolve` con la query dada y devuelve un solo Track.
    pub async fn resolve(&self, query: &str) -> Result<Track, MicroserviceError> {
        let payload = json!({
            "action": "resolve",
            "query": query
        }).to_string() + "\n";

        let raw = self.send_raw(&payload).await?;

        // Deserializamos inyectando `Track` en el genérico
        let response: ApiResponse<Track> = serde_json::from_str(&raw)
            .map_err(|e| MicroserviceError::InvalidResponse(format!("Fallo parseo resolve: {}. Raw: {}", e, raw)))?;

        if response.status == "ok" {
            response.data.ok_or_else(|| MicroserviceError::ServiceError("El microservicio devolvió ok pero 'data' es null".into()))
        } else {
            Err(MicroserviceError::ServiceError(
                response.message.unwrap_or_else(|| "Error desconocido del microservicio".into())
            ))
        }
    }

    /// Envía una acción `radio` con la query dada y devuelve una lista de Tracks.
    pub async fn radio(&self, query: &str) -> Result<Vec<Track>, MicroserviceError> {
        let payload = json!({
            "action": "radio",
            "query": query
        }).to_string() + "\n";

        let raw = self.send_raw(&payload).await?;

        // Deserializamos inyectando `Vec<Track>` en el genérico
        let response: ApiResponse<Vec<Track>> = serde_json::from_str(&raw)
            .map_err(|e| MicroserviceError::InvalidResponse(format!("Fallo parseo radio: {}. Raw: {}", e, raw)))?;

        if response.status == "ok" {
            response.data.ok_or_else(|| MicroserviceError::ServiceError("El microservicio devolvió ok pero 'data' es null".into()))
        } else {
            Err(MicroserviceError::ServiceError(
                response.message.unwrap_or_else(|| "Error al generar la radio desde el microservicio".into())
            ))
        }
    }

    pub async fn mark_as_played(&self, track_id: &str) -> Result<(), MicroserviceError> {
        let payload = serde_json::json!({
            "action": "played",
            "query": track_id
        }).to_string() + "\n";

        let raw = self.send_raw(&payload).await?;

        // Parseamos usando serde_json::Value ya que 'data' vendrá vacío o no nos importa
        let response: ApiResponse<serde_json::Value> = serde_json::from_str(&raw)
            .map_err(|e| MicroserviceError::InvalidResponse(format!("Fallo parseo en mark_as_played: {}", e)))?;

        if response.status == "ok" {
            Ok(())
        } else {
            Err(MicroserviceError::ServiceError(
                response.message.unwrap_or_else(|| "Error al registrar el play en el microservicio".into())
            ))
        }
    }

    /// Abre conexión, envía payload, lee UNA SOLA LÍNEA de respuesta.
    async fn send_raw(&self, payload: &str) -> Result<String, MicroserviceError> {
        let mut stream = TcpStream::connect(&self.addr)
            .await
            .map_err(MicroserviceError::ConnectionFailed)?;

        stream.write_all(payload.as_bytes())
            .await
            .map_err(MicroserviceError::IoError)?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();

        reader.read_line(&mut response)
            .await
            .map_err(MicroserviceError::IoError)?;

        Ok(response.trim().to_string())
    }
}