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

    pub fn set_request(&mut self, request: &GameRequest) -> bool {
        match request {
            GameRequest::Draw(offer) => {
                if self.draw_offered == *offer {
                    return false;
                }
                self.draw_offered = *offer;
            }
            GameRequest::Undo(request) => {
                if self.undo_requested == *request {
                    return false;
                }
                self.undo_requested = *request;
            }
            GameRequest::MoreTime(duration) => {
                if self.more_time_offered == *duration {
                    return false;
                }
                self.more_time_offered = *duration;
            }
        }
        return true;
    }

    pub fn accept_draw_request(&mut self) -> bool {
        if !self.draw_offered {
            return false;
        }
        self.draw_offered = false;
        true
    }

    pub fn accept_undo_request(&mut self) -> bool {
        if !self.undo_requested {
            return false;
        }
        self.undo_requested = false;
        true
    }

    pub fn accept_more_time_request(&mut self) -> Option<Duration> {
        if self.more_time_offered.is_none() {
            return None;
        }
        let duration = self.more_time_offered.take();
        duration
    }
}
