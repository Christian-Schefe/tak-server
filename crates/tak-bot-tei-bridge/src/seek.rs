use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub struct SeekService {
    seeks: Arc<Mutex<HashSet<String>>>,
}

impl SeekService {
    pub fn new() -> Self {
        Self {
            seeks: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn begin_seek(&self, seek_id: String) {
        self.seeks.lock().unwrap().insert(seek_id);
    }

    pub fn end_seek(&self, seek_id: String) -> bool {
        self.seeks.lock().unwrap().remove(&seek_id)
    }
}
