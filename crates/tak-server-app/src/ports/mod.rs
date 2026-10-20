pub mod authentication;
pub mod chat;
pub mod connection;
pub mod email;
pub mod event;
pub mod game_history;
pub mod matches;
pub mod moderation;
pub mod notification;
pub mod profile;
pub mod puzzle;
pub mod rating;
pub mod stats;
pub mod tournament;
pub mod transaction;

#[derive(Debug, Clone)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl Pagination {
    pub fn new(page_index: usize, page_size: usize) -> Self {
        let offset = Some(page_index * page_size);
        let limit = Some(page_size);
        Self { offset, limit }
    }
}

pub struct PaginatedResponse<T> {
    pub total_count: usize,
    pub items: Vec<T>,
}

impl<T> PaginatedResponse<T> {
    pub fn map<U, F>(self, f: F) -> PaginatedResponse<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResponse {
            total_count: self.total_count,
            items: self.items.into_iter().map(f).collect(),
        }
    }
}
