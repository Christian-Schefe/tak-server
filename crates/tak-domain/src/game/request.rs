use std::time::Duration;

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

impl GameRequests {
    pub fn new() -> Self {
        GameRequests {
            draw_offered: false,
            undo_requested: false,
            more_time_offered: None,
        }
    }

    pub fn set_request(&mut self, request: GameRequest) -> Option<GameRequest> {
        match request {
            GameRequest::Draw(offer) => {
                if self.draw_offered == offer {
                    return None;
                }
                self.draw_offered = offer;
            }
            GameRequest::Undo(request) => {
                if self.undo_requested == request {
                    return None;
                }
                self.undo_requested = request;
            }
            GameRequest::MoreTime(duration) => {
                if self.more_time_offered == duration {
                    return None;
                }
                self.more_time_offered = duration;
            }
        }
        Some(request)
    }

    pub fn consume_request(&mut self, request_type: GameRequestType) -> GameRequest {
        match request_type {
            GameRequestType::Draw => {
                let result = GameRequest::Draw(self.draw_offered);
                self.draw_offered = false;
                result
            }
            GameRequestType::Undo => {
                let result = GameRequest::Undo(self.undo_requested);
                self.undo_requested = false;
                result
            }
            GameRequestType::MoreTime => {
                let result = GameRequest::MoreTime(self.more_time_offered);
                self.more_time_offered = None;
                result
            }
        }
    }
}
