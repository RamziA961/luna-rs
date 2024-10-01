use std::collections::VecDeque;

use tracing::trace;

use super::{QueueElement, VideoMetadata};

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    playing: bool,
    current_track: Option<VideoMetadata>,
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

    pub fn set_current_track(&mut self, current_track: &Option<VideoMetadata>) {
        self.current_track = current_track.clone();
    }

    pub fn enqueue(&mut self, element: QueueElement) {
        self.queue.push_back(element)
    }

    pub fn next(&self) -> Option<&QueueElement> {
        self.queue.front()
    }

    pub fn dequeue(&mut self) -> Option<VideoMetadata> {
        let mut should_pop = false;
        let next = match self.queue.front_mut() {
            Some(QueueElement::Track(t)) => {
                should_pop = true;
                Some(t.clone())
            }
            Some(QueueElement::Playlist(p)) => p.items.pop_front(),
            None => None,
        };

        if should_pop {
            self.queue.pop_front();
        }

        next
    }

    pub fn play_next(&mut self) {
        let next = self.dequeue();
        trace!(next_track=?next);
        self.set_current_track(&next);
        self.set_playing(next.is_some());
    }
}
