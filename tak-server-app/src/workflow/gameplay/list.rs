use std::sync::Arc;

use crate::{domain::game::GameService, workflow::gameplay::GameView};

pub trait ListOngoingGameUseCase {
    fn list_games(&self) -> Vec<GameView>;
}

pub struct ListOngoingGameUseCaseImpl<G: GameService> {
    game_service: Arc<G>,
}

impl<G: GameService> ListOngoingGameUseCaseImpl<G> {
    pub fn new(game_service: Arc<G>) -> Self {
        Self { game_service }
    }
}

impl<G: GameService> ListOngoingGameUseCase for ListOngoingGameUseCaseImpl<G> {
    fn list_games(&self) -> Vec<GameView> {
        self.game_service
            .get_games()
            .into_iter()
            .map(|game| GameView::from(game))
            .collect()
    }
}
