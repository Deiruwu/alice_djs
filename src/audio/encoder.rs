use songbird::input::{File, Input};
use std::path::Path;

/// Formatos que Discord acepta de forma nativa sin recodificar.
/// Cualquier otro formato pasara por ffmpeg internamente via songbird.
const OPUS_NATIVE: &[&str] = &[".ogg", ".opus", ".webm"];

/// Errores del encoder.
#[derive(Debug)]
pub enum EncoderError {
    /// El archivo no existe en el path dado.
    FileNotFound(String),
    /// La extension del archivo no esta soportada.
    UnsupportedFormat(String),
}

impl std::fmt::Display for EncoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::FileNotFound(p)      => write!(f, "Archivo no encontrado: {}", p),
            Self::UnsupportedFormat(p) => write!(f, "Formato no soportado: {}", p),
        }
    }
}

impl std::error::Error for EncoderError {}

/// Convierte un file_path en un Input listo para songbird.
///
/// Formatos nativos (.ogg, .opus, .webm): se entregan directamente.
/// Resto (.m4a, .flac, mp3, etc.): songbird los pasa por ffmpeg internamente.
///
/// REQUISITO: ffmpeg debe estar instalado en el sistema para formatos no nativos.
pub fn create_input(file_path: &str) -> Result<Input, EncoderError> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(EncoderError::FileNotFound(file_path.to_string()));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();

    // Formatos soportados explicitamente.
    // .m4a y .flac van por ffmpeg (no son nativos de opus).
    // FUTURO: cuando symphonia este integrado, los formatos compatibles
    //         podran ir por symphonia en lugar de ffmpeg.
    let supported = matches!(
        ext.as_str(),
        ".ogg" | ".opus" | ".webm" | ".m4a" | ".flac" | ".mp3" | ".wav"
    );

    if !supported {
        return Err(EncoderError::UnsupportedFormat(ext));
    }

    Ok(File::new(file_path.to_string()).into())
}