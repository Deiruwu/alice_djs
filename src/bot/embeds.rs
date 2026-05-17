use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::Color;
use crate::model::{Track, Artist, TrackState};
use crate::audio::QueuedTrack;

// ─── Colores ──────────────────────────────────────────────────────────────────

pub const COLOR_PLAYING: Color = Color::from_rgb(180, 160, 255);

pub const COLOR_QUEUED: Color = Color::from_rgb(120, 190, 170);
pub const COLOR_SKIP:     Color = Color::from_rgb(254, 231, 92);  // amarillo
pub const COLOR_FINISHED: Color = Color::from_rgb(128, 128, 128); // gris

pub const COLOR_RADIO: Color = Color::from_rgb(255, 105, 180); // Un rosa neón/synthwave para la radio

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
    pub author_icon_url: Option<&'a str>,
    pub position:     Option<usize>,
    pub color:        Color,
    pub title_prefix: &'a str,
}

pub fn build_track_embed(opts: TrackEmbedOptions<'_>) -> CreateEmbed {
    let TrackEmbedOptions { track, requested_by, author_icon_url, position, color, title_prefix } = opts;

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

    embed = embed.field("⏱ Duración", format!("{}  `{}`", bar, duration), false);

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

    let mut footer = CreateEmbedFooter::new(format!("Request by {}", requested_by));
    if let Some(icon_url) = author_icon_url {
        footer = footer.icon_url(icon_url);
    }

    embed.footer(footer)
}

pub fn build_queue_embed(track: &Track, requested_by: &str, author_icon_url: Option<&str>, position: usize) -> CreateEmbed {
    let artists  = fmt_artists(&track.artists);
    let duration = fmt_duration(track.duration_seconds);

    let mut embed = CreateEmbed::new()
        .color(COLOR_QUEUED)
        .title(format!("**{}** — {}", track.title, artists))
        .description(format!(
            "Posición en cola: #{} ⏱ `{}`",
            position.to_string(),
            duration,
        ));

    if let Some(ref thumb) = track.thumbnail_url {
        embed = embed.thumbnail(thumb.clone());
    }

    let mut footer = CreateEmbedFooter::new(format!("Request by {}", requested_by));
    if let Some(icon_url) = author_icon_url {
        footer = footer.icon_url(icon_url);
    }

    embed.footer(footer)
}

pub fn build_full_queue_embed(current: Option<&QueuedTrack>, queue: &[QueuedTrack], limit: usize) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .color(COLOR_QUEUED)
        .title("📋 Cola de Reproducción");

    // 1. Canción actual
    if let Some(c) = current {
        let artists = fmt_artists(&c.track.artists);
        let duration = fmt_duration(c.track.duration_seconds);
        embed = embed.field(
            "Reproduciendo Ahora",
            format!("**{}** — {}\n`{}` • Pedido por: {}", c.track.title, artists, duration, c.requested_by),
            false,
        );
    }

    if queue.is_empty() {
        embed = embed.description("No hay más canciones en la cola.");
    } else {
        let mut queue_str = String::new();

        for (i, q_track) in queue.iter().take(limit).enumerate() {
            let artists = fmt_artists(&q_track.track.artists);
            let duration = fmt_duration(q_track.track.duration_seconds);
            let state = match q_track.track.state {
                TrackState::Cached => "📁",
                TrackState::Partial => ""
            };

            queue_str.push_str(&format!(
                "`{}.` **{}** — {} {}\n`{}` • {}\n\n",
                i + 1,
                q_track.track.title,
                artists,
                state,
                duration,
                q_track.requested_by
            ));
        }

        if queue.len() > limit {
            let remaining = queue.len() - limit;
            queue_str.push_str(&format!("*...y {} canciones más.*", remaining));
        }

        embed = embed.field("Siguientes en cola", queue_str, false);
    }

    embed
}

pub struct RadioEmbedOptions<'a> {
    pub seed_track:      &'a Track,
    pub enqueued_count:  usize,
    pub requested_by:    &'a str,
    pub author_icon_url: Option<&'a str>,
}

pub fn build_radio_embed(opts: RadioEmbedOptions<'_>) -> CreateEmbed {
    let RadioEmbedOptions { seed_track, enqueued_count, requested_by, author_icon_url } = opts;
    let artists = fmt_artists(&seed_track.artists);

    let mut embed = CreateEmbed::new()
        .color(COLOR_RADIO)
        .title("📻 Estación de Radio Iniciada")
        .description(format!(
            "Se ha generado un mix automático basado en:\n**{}** — {}",
            seed_track.title, artists
        ))
        .field(
            "📊 Estado de la Cola",
            format!("Se añadieron **{}** canciones generadas dinámicamente.", enqueued_count),
            false
        );

    if let Some(ref url) = seed_track.thumbnail_url {
        embed = embed.thumbnail(url.clone());
    }

    let mut footer = CreateEmbedFooter::new(format!("Iniciada por {}", requested_by));
    if let Some(icon_url) = author_icon_url {
        footer = footer.icon_url(icon_url);
    }

    embed.footer(footer)
}