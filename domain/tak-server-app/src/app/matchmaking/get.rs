use std::sync::Arc;

use crate::{
    app::matchmaking::SeekView,
    domain::{SeekId, seek::SeekService},
};

pub trait GetSeekUseCase {
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView>;
}

pub struct GetSeekUseCaseImpl<S: SeekService> {
    seek_service: Arc<S>,
}

impl<S: SeekService> GetSeekUseCaseImpl<S> {
    pub fn new(seek_service: Arc<S>) -> Self {
        Self { seek_service }
    }
}

impl<S: SeekService> GetSeekUseCase for GetSeekUseCaseImpl<S> {
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView> {
        self.seek_service.get_seek(seek_id).map(SeekView::from)
    }
}
