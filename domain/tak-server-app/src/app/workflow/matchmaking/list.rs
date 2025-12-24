use std::sync::Arc;

use crate::app::{domain::seek::SeekService, workflow::matchmaking::SeekView};

pub trait ListSeeksUseCase {
    fn list_seeks(&self) -> Vec<SeekView>;
}

pub struct ListSeeksUseCaseImpl<S: SeekService> {
    seek_service: Arc<S>,
}

impl<S: SeekService> ListSeeksUseCaseImpl<S> {
    pub fn new(seek_service: Arc<S>) -> Self {
        Self { seek_service }
    }
}

impl<S: SeekService> ListSeeksUseCase for ListSeeksUseCaseImpl<S> {
    fn list_seeks(&self) -> Vec<SeekView> {
        let res = self
            .seek_service
            .list_seeks()
            .into_iter()
            .map(SeekView::from)
            .collect();
        res
    }
}
