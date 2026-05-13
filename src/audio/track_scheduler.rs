use std::collections::VecDeque;
use crate::model::Track;

/// Una entrada en la cola: la cancion y quien la pidio.
#[derive(Debug, Clone)]
pub struct QueuedTrack {
    pub track:     Track,
    /// Nombre del usuario que añadio la cancion.
    pub requested_by: String,
}

/// Gestiona la cola de reproduccion para un guild concreto.
///
/// Un TrackScheduler por guild; el MusicManager los indexa por GuildId.
pub struct TrackScheduler {
    queue:   VecDeque<QueuedTrack>,
    current: Option<QueuedTrack>,
}

impl TrackScheduler {
    pub fn new() -> Self {
        Self {
            queue:   VecDeque::new(),
            current: None,
        }
    }

    /// Añade una cancion al final de la cola.
    pub fn enqueue(&mut self, track: Track, requested_by: String) {
        self.queue.push_back(QueuedTrack { track, requested_by });
    }

    /// Saca la siguiente cancion de la cola y la pone como current.
    /// Devuelve None si la cola esta vacia.
    pub fn next(&mut self) -> Option<&QueuedTrack> {
        self.current = self.queue.pop_front();
        self.current.as_ref()
    }

    /// La cancion que se esta reproduciendo ahora mismo.
    pub fn current(&self) -> Option<&QueuedTrack> {
        self.current.as_ref()
    }

    /// Cuantas canciones hay en cola (sin contar la actual).
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Lista de canciones en cola en orden, sin consumirla.
    pub fn list(&self) -> impl Iterator<Item = &QueuedTrack> {
        self.queue.iter()
    }

    /// Vacia la cola completa. No toca current.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Elimina la cancion en la posicion dada (0 = primera en cola).
    /// Devuelve la cancion eliminada, o None si el indice esta fuera de rango.
    pub fn remove(&mut self, index: usize) -> Option<QueuedTrack> {
        self.queue.remove(index)
    }
}