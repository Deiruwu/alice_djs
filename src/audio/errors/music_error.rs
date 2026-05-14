use thiserror::Error;

#[derive(Debug, Error)]
pub enum MusicError {
    #[error("Error al resolver la pista: {0}")]
    ResolveError(String),

    #[error("La pista no tiene ruta de archivo")]
    NoFilePath,

    #[error("Error al codificar audio: {0}")]
    EncoderError(String),

    #[error("No hay ningún scheduler activo para este servidor")]
    NotPlaying,

    #[error("La posición indicada está fuera del rango de la cola")]
    OutOfRange,
}