use std::time::Duration;

use tak_core::TakPlayer;

#[derive(Clone, Debug)]
pub enum GameRequest {
    Draw(bool),
    Undo(bool),
    MoreTime(Option<Duration>),
}

#[derive(Clone, Debug)]
pub enum GameRequestType {
    Draw,
    Undo,
    MoreTime,
}

#[derive(Clone, Debug)]
pub struct GameRequests {
    pub draw_offered: bool,
    pub undo_requested: bool,
    pub more_time_offered: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct GameRequestSystem {
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

impl GameRequestSystem {
    pub fn new() -> Self {
        GameRequestSystem {
            white_requests: GameRequests {
                draw_offered: false,
                undo_requested: false,
                more_time_offered: None,
            },
            black_requests: GameRequests {
                draw_offered: false,
                undo_requested: false,
                more_time_offered: None,
            },
        }
    }

    pub fn set_request(&mut self, player: TakPlayer, request: GameRequest) -> Option<GameRequest> {
        let requests = match player {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        match request {
            GameRequest::Draw(offer) => {
                if requests.draw_offered == offer {
                    return None;
                }
                requests.draw_offered = offer;
            }
            GameRequest::Undo(request) => {
                if requests.undo_requested == request {
                    return None;
                }
                requests.undo_requested = request;
            }
            GameRequest::MoreTime(duration) => {
                if requests.more_time_offered == duration {
                    return None;
                }
                requests.more_time_offered = duration;
            }
        }

        Some(request)
    }

    pub fn consume_request(
        &mut self,
        player: TakPlayer,
        request_type: GameRequestType,
    ) -> GameRequest {
        let requests = match player {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        let result = match request_type {
            GameRequestType::Draw => {
                let result = GameRequest::Draw(requests.draw_offered);
                requests.draw_offered = false;
                result
            }
            GameRequestType::Undo => {
                let result = GameRequest::Undo(requests.undo_requested);
                requests.undo_requested = false;
                result
            }
            GameRequestType::MoreTime => {
                let result = GameRequest::MoreTime(requests.more_time_offered);
                requests.more_time_offered = None;
                result
            }
        };
        result
    }
}
