use std::collections::HashMap;

use crate::{TakPlayer, TakRequest, TakRequestId, TakRequestType};

#[derive(Clone, Debug)]
pub struct TakRequestSystem {
    pub requests: HashMap<TakRequestId, TakRequest>,
    next_request_id: u64,
}

impl TakRequestSystem {
    pub fn new() -> Self {
        TakRequestSystem {
            requests: HashMap::new(),
            next_request_id: 0,
        }
    }

    pub fn get_request(&self, request_id: TakRequestId) -> Option<TakRequest> {
        self.requests.get(&request_id).cloned()
    }

    pub fn get_all_requests(&self) -> Vec<TakRequest> {
        self.requests.values().cloned().collect()
    }

    pub fn add_request(
        &mut self,
        player: &TakPlayer,
        request: TakRequestType,
    ) -> Option<TakRequest> {
        if self.requests.iter().any(|(_, r)| {
            r.player == *player
                && match (&r.request_type, &request) {
                    (TakRequestType::Draw, TakRequestType::Draw) => true,
                    (TakRequestType::Undo, TakRequestType::Undo) => true,
                    (TakRequestType::MoreTime(_), TakRequestType::MoreTime(_)) => true,
                    _ => false,
                }
        }) {
            None
        } else {
            let id = TakRequestId(self.next_request_id);
            let request = TakRequest {
                id,
                player: *player,
                request_type: request,
            };
            self.requests.insert(id, request.clone());
            self.next_request_id += 1;
            Some(request)
        }
    }

    pub fn take_request_if(
        &mut self,
        request_id: TakRequestId,
        predicate: impl Fn(&TakRequest) -> bool,
    ) -> Option<TakRequest> {
        if let Some(request) = self.requests.get(&request_id) {
            if predicate(request) {
                let request = self.requests.remove(&request_id).unwrap();
                return Some(request);
            }
        }
        None
    }
}
