use crate::model::Track;
pub enum PlaybackStatus {
    PlayingNow(Track),
    Enqueued {
        track: Track,
        position: usize,
    },
}