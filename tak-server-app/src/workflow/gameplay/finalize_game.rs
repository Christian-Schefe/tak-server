use std::sync::Arc;

use crate::{
    domain::{
        game::Game,
        game_history::{GameHistoryService, GameRepository},
        r#match::MatchService,
        rating::{RatingRepository, RatingService},
        spectator::SpectatorService,
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    workflow::player::notify_player::NotifyPlayerWorkflow,
};

#[async_trait::async_trait]
pub trait FinalizeGameWorkflow {
    async fn finalize_game(&self, ended_game: Game);
}

pub struct FinalizeGameWorkflowImpl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
> {
    game_repository: Arc<G>,
    rating_service: Arc<R>,
    rating_repository: Arc<RP>,
    game_history_service: Arc<GH>,
    match_service: Arc<M>,
    notify_player_workflow: Arc<NP>,
    spectator_service: Arc<SPS>,
    listener_notification_port: Arc<L>,
}

impl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
> FinalizeGameWorkflowImpl<G, R, RP, GH, M, NP, SPS, L>
{
    pub fn new(
        game_repository: Arc<G>,
        rating_service: Arc<R>,
        rating_repository: Arc<RP>,
        game_history_service: Arc<GH>,
        match_service: Arc<M>,
        notify_player_workflow: Arc<NP>,
        spectator_service: Arc<SPS>,
        listener_notification_port: Arc<L>,
    ) -> Self {
        Self {
            game_repository,
            rating_service,
            rating_repository,
            game_history_service,
            match_service,
            notify_player_workflow,
            spectator_service,
            listener_notification_port,
        }
    }
}

#[async_trait::async_trait]
impl<
    G: GameRepository + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    RP: RatingRepository + Send + Sync + 'static,
    GH: GameHistoryService + Send + Sync + 'static,
    M: MatchService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    SPS: SpectatorService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
> FinalizeGameWorkflow for FinalizeGameWorkflowImpl<G, R, RP, GH, M, NP, SPS, L>
{
    async fn finalize_game(&self, ended_game: Game) {
        let over_msg = ListenerMessage::GameOver {
            game_id: ended_game.game_id,
            game_state: ended_game.game.game_state(),
        };

        self.notify_player_workflow
            .notify_players(
                &[ended_game.white_id, ended_game.black_id],
                over_msg.clone(),
            )
            .await;

        let observers = self
            .spectator_service
            .get_spectators_for_game(ended_game.game_id);
        self.listener_notification_port
            .notify_listeners(&observers, over_msg);

        self.spectator_service.remove_game(ended_game.game_id);

        let ended_game_clone = ended_game.clone();
        let rating_service = self.rating_service.clone();
        let game_rating_info = match self
            .rating_repository
            .update_player_ratings(
                ended_game.white_id,
                ended_game.black_id,
                move |mut w_rating, mut b_rating| {
                    let res = rating_service.calculate_ratings(
                        &ended_game_clone,
                        &mut w_rating,
                        &mut b_rating,
                    );
                    (w_rating, b_rating, res)
                },
            )
            .await
        {
            Ok(info) => info,
            Err(e) => {
                log::error!(
                    "Failed to update player ratings for game {}: {}",
                    ended_game.game_id,
                    e
                );
                None
            }
        };

        let game_id = ended_game.game_id;
        let match_id = ended_game.match_id;
        let game_record_update = self
            .game_history_service
            .get_finished_game_record_update(ended_game, game_rating_info);

        self.match_service.end_game_in_match(match_id);

        if let Err(e) = self
            .game_repository
            .update_finished_game(game_id, game_record_update)
            .await
        {
            log::error!(
                "Failed to update finished game record for game {}: {}",
                game_id,
                e
            );
        }
    }
}
