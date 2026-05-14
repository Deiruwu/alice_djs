use songbird::input::{File, Input, RawAdapter};
use std::io::{Read, Seek, SeekFrom, Result as IoResult};
use std::path::Path;
use std::process::{Command, Stdio};
use symphonia_core::io::MediaSource;

#[derive(Debug)]
pub enum EncoderError {
    FileNotFound(String),
    UnsupportedFormat(String),
    SpawnError(String),
}

impl std::fmt::Display for EncoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::FileNotFound(p)      => write!(f, "Archivo no encontrado: {}", p),
            Self::UnsupportedFormat(p) => write!(f, "Formato no soportado: {}", p),
            Self::SpawnError(e)        => write!(f, "Error al lanzar FFmpeg: {}", e),
        }
    }
}

impl std::error::Error for EncoderError {}

// ─── Wrapper MediaSource para streams no seekables ───────────────────────────

struct ReadOnlyMediaSource<R: Read + Send + Sync> {
    inner: R,
}

impl<R: Read + Send + Sync> Read for ReadOnlyMediaSource<R> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.inner.read(buf)
    }
}

impl<R: Read + Send + Sync> Seek for ReadOnlyMediaSource<R> {
    fn seek(&mut self, _pos: SeekFrom) -> IoResult<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "stream no seekable",
        ))
    }
}

impl<R: Read + Send + Sync + 'static> MediaSource for ReadOnlyMediaSource<R> {
    fn is_seekable(&self) -> bool { false }
    fn byte_len(&self) -> Option<u64> { None }
}

// ─── API pública ─────────────────────────────────────────────────────────────

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

    let supported = matches!(
        ext.as_str(),
        ".ogg" | ".opus" | ".webm" | ".m4a" | ".flac" | ".mp3" | ".wav"
    );

    if !supported {
        return Err(EncoderError::UnsupportedFormat(ext));
    }

    // .flac va siempre por FFmpeg para garantizar resample a 48000 Hz.
    if ext == ".flac" {
        return spawn_ffmpeg_input(file_path);
    }

    Ok(File::new(file_path.to_string()).into())
}

// ─── FFmpeg pipe con RawAdapter ───────────────────────────────────────────────

fn spawn_ffmpeg_input(file_path: &str) -> Result<Input, EncoderError> {
    let mut child = Command::new("ffmpeg")
        .args([
            "-i",        file_path,
            "-f",        "f32le",
            "-ac",       "2",
            "-ar",       "48000",
            "-loglevel", "quiet",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| EncoderError::SpawnError(e.to_string()))?;

    let stdout = child.stdout.take()
        .ok_or_else(|| EncoderError::SpawnError("FFmpeg no abrió stdout".to_string()))?;

    let source  = ReadOnlyMediaSource { inner: stdout };
    let adapter = RawAdapter::new(source, 48000, 2);

    Ok(Input::from(adapter))
}