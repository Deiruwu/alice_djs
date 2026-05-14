use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::Color;
use crate::model::{Track, Artist};

// ─── Colores ──────────────────────────────────────────────────────────────────

pub const COLOR_PLAYING: Color = Color::from_rgb(180, 160, 255);

pub const COLOR_QUEUED: Color = Color::from_rgb(120, 190, 170);
pub const COLOR_SKIP:     Color = Color::from_rgb(254, 231, 92);  // amarillo
pub const COLOR_FINISHED: Color = Color::from_rgb(128, 128, 128); // gris

// ─── Helpers de formato ───────────────────────────────────────────────────────

pub fn fmt_duration(secs: i32) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

pub fn fmt_artists(artists: &[Artist]) -> String {
    if artists.is_empty() {
        "Desconocido".to_string()
    } else {
        artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(" & ")
    }
}

/// Barra de progreso visual, solo decorativa (no tenemos posición en tiempo real).
/// Se usa para mostrar la duración de forma más visual.
pub fn duration_bar(secs: i32) -> String {
    // Escala: cada bloque = 30 segundos, máximo 20 bloques
    let blocks = ((secs / 30) as usize).min(20).max(1);
    let filled = "█".repeat(blocks);
    let empty  = "░".repeat(20usize.saturating_sub(blocks));
    format!("{}{}", filled, empty)
}

// ─── Embed principal de track ─────────────────────────────────────────────────

pub struct TrackEmbedOptions<'a> {
    pub track:        &'a Track,
    pub requested_by: &'a str,
    pub position:     Option<usize>,   // None = reproduciendo ahora
    pub color:        Color,
    pub title_prefix: &'a str,         // "🎵 Reproduciendo", "⏭ Siguiente en cola", etc.
}

pub fn build_track_embed(opts: TrackEmbedOptions<'_>) -> CreateEmbed {
    let TrackEmbedOptions { track, requested_by, position, color, title_prefix } = opts;

    let artists   = fmt_artists(&track.artists);
    let duration  = fmt_duration(track.duration_seconds);
    let bar       = duration_bar(track.duration_seconds);

    let mut embed = CreateEmbed::new()
        .color(color)
        .title(format!("{} — {}", title_prefix, track.title))
        .description(format!("**{}**", artists));

    // Thumbnail de álbum si existe
    if let Some(ref url) = track.thumbnail_url {
        embed = embed.thumbnail(url.clone());
    }

    // ── Campos principales ────────────────────────────────────────────────────

    embed = embed.field("⏱ Duración", format!("`{}` {}", duration, bar), false);

    if let Some(ref album) = track.album {
        embed = embed.field("💿 Álbum", &album.name, true);
    }

    // BPM y clave Camelot en la misma fila si ambos existen
    match (track.bpm, track.camelot_key.as_deref()) {
        (Some(bpm), Some(key)) => {
            embed = embed
                .field("BPM",   bpm.to_string(), true)
                .field("Clave", key,              true);
        }
        (Some(bpm), None) => {
            embed = embed.field("BPM", bpm.to_string(), true);
        }
        (None, Some(key)) => {
            embed = embed.field("Clave", key, true);
        }
        _ => {}
    }

    // Posición en cola
    if let Some(pos) = position {
        embed = embed.field("📋 Posición en cola", format!("#{}", pos), true);
    }

    // Footer con quien lo pidió
    embed = embed.footer(
        CreateEmbedFooter::new(format!("Pedido por {}", requested_by))
    );

    embed
}

pub fn build_queue_embed(
    track: &Track,
    requested_by: &str,
    position: usize,
) -> CreateEmbed {
    let artists  = fmt_artists(&track.artists);
    let duration = fmt_duration(track.duration_seconds);

    let mut embed = CreateEmbed::new()
        .color(COLOR_QUEUED)
        .title(format!("📋 Añadido a la cola • #{}", position))
        .description(format!(
            "**{}** — {}\n⏱ `{}`",
            track.title,
            artists,
            duration
        ));

    if let Some(ref thumb) = track.thumbnail_url {
        embed = embed.thumbnail(thumb.clone());
    }

    embed.footer(
        CreateEmbedFooter::new(format!("Pedido por {}", requested_by))
    )
}