use crate::workflow::gameplay::OngoingGameView;
use std::sync::Arc;
use tak_domain::game::GameService;

pub trait ListOngoingGameUseCase {
    fn list_games(&self) -> Vec<OngoingGameView>;
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
    fn list_games(&self) -> Vec<OngoingGameView> {
        self.game_service
            .get_games()
            .into_iter()
            .map(|game| OngoingGameView::from(game))
            .collect()
    }
}
