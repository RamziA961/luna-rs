use std::collections::VecDeque;

use songbird::tracks::TrackHandle;
use tracing::trace;

use super::{QueueElement, VideoMetadata};

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    playing: bool,
    current_track: Option<VideoMetadata>,
    track_handle: Option<TrackHandle>,
    queue: VecDeque<QueueElement>,
}

impl PlaybackState {
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn set_playing(&mut self, play_state: bool) {
        self.playing = play_state;
    }

    pub fn get_current_track(&self) -> &Option<VideoMetadata> {
        &self.current_track
    }

    pub fn set_current_track(&mut self, current_track: Option<VideoMetadata>) {
        self.current_track = current_track;
    }

    pub fn set_track_handle(&mut self, track_handle: Option<TrackHandle>) {
        self.track_handle = track_handle.clone();
    }

    pub fn get_track_handle(&self) -> &Option<TrackHandle> {
        &self.track_handle
    }

    pub fn get_track_handle_mut(&mut self) -> &mut Option<TrackHandle> {
        &mut self.track_handle
    }

    pub fn enqueue(&mut self, element: QueueElement) {
        self.queue.push_back(element)
    }

    pub fn next(&self) -> Option<&QueueElement> {
        self.queue.front()
    }

    pub fn dequeue(&mut self) -> Option<VideoMetadata> {
        match self.queue.pop_front() {
            Some(QueueElement::Track(t)) => Some(t),
            Some(QueueElement::Playlist(mut p)) => {
                let next = p.items.pop_front();
                if !p.items.is_empty() {
                    self.queue.push_front(QueueElement::Playlist(p));
                }
                next
            }
            None => None,
        }
    }

    pub fn play_next(&mut self) {
        let next = self.dequeue();
        trace!(next_track=?next);
        self.set_playing(next.is_some());
        self.set_current_track(next);
        self.set_track_handle(None)
    }

    pub fn reset(&mut self) {
        self.set_current_track(None);
        self.set_track_handle(None);
        self.set_playing(false);
        self.queue.clear();
    }
}
