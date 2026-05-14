use std::process::{Command, Stdio};
use songbird::input::{ChildContainer, Input};

pub struct AudioUtils;

impl AudioUtils {
    pub fn spawn_normalized_stream(file_path: &str) -> Result<Input, std::io::Error> {
        let child = Command::new("ffmpeg")
            .args([
                "-i", file_path,
                "-f", "f32le",        // <-- f32le, no s16le
                "-ac", "2",
                "-ar", "48000",
                "-af", "loudnorm=I=-16:TP=-1.5:LRA=11",
                "-loglevel", "quiet",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(ChildContainer::from(child).into())
    }
}