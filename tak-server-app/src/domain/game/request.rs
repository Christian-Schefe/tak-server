use std::{collections::HashMap, time::Duration};

use tak_core::TakPlayer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameRequestId(pub u64);

#[derive(Clone, Debug)]
pub enum GameRequestType {
    Draw,
    Undo,
    MoreTime(Duration),
}

#[derive(Clone, Debug)]
pub struct GameRequest {
    pub id: GameRequestId,
    pub player: TakPlayer,
    pub request_type: GameRequestType,
}

#[derive(Clone, Debug)]
pub struct GameRequestSystem {
    pub requests: HashMap<GameRequestId, GameRequest>,
    next_request_id: u64,
}

impl GameRequestSystem {
    pub fn new() -> Self {
        GameRequestSystem {
            requests: HashMap::new(),
            next_request_id: 0,
        }
    }

    pub fn get_request(&self, request_id: GameRequestId) -> Option<GameRequest> {
        self.requests.get(&request_id).cloned()
    }

    pub fn get_all_requests(&self) -> Vec<GameRequest> {
        self.requests.values().cloned().collect()
    }

    pub fn add_request(
        &mut self,
        player: &TakPlayer,
        request: GameRequestType,
    ) -> Option<GameRequest> {
        if self.requests.iter().any(|(_, r)| {
            r.player == *player
                && match (&r.request_type, &request) {
                    (GameRequestType::Draw, GameRequestType::Draw) => true,
                    (GameRequestType::Undo, GameRequestType::Undo) => true,
                    (GameRequestType::MoreTime(_), GameRequestType::MoreTime(_)) => true,
                    _ => false,
                }
        }) {
            None
        } else {
            let id = GameRequestId(self.next_request_id);
            let request = GameRequest {
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
        request_id: GameRequestId,
        predicate: impl Fn(&GameRequest) -> bool,
    ) -> Option<GameRequest> {
        if let Some(request) = self.requests.get(&request_id) {
            if predicate(request) {
                let request = self.requests.remove(&request_id).unwrap();
                return Some(request);
            }
        }
        None
    }
}
