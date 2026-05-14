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
