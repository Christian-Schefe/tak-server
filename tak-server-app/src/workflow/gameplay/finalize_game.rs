use std::sync::Arc;

use tak_core::{TakGameOverState, TakPlayer};

use crate::{
    domain::{
        game::FinishedGame,
        game_history::{GameHistoryService, GameRatingInfo, GameRepository},
        r#match::MatchService,
        rating::{PlayerRating, RatingRepository, RatingService},
        spectator::SpectatorService,
        stats::{GameOutcome, StatsRepository},
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    workflow::{
        account::get_account::GetAccountWorkflow, gameplay::FinishedGameView,
        player::notify_player::NotifyPlayerWorkflow,
    },
};

#[async_trait::async_trait]
pub trait FinalizeGameWorkflow {
    async fn finalize_game(&self, ended_game: FinishedGame);
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
    A: GetAccountWorkflow,
    S: StatsRepository,
> {
    game_repository: Arc<G>,
    rating_service: Arc<R>,
    rating_repository: Arc<RP>,
    game_history_service: Arc<GH>,
    match_service: Arc<M>,
    notify_player_workflow: Arc<NP>,
    spectator_service: Arc<SPS>,
    listener_notification_port: Arc<L>,
    get_account_workflow: Arc<A>,
    stats_repository: Arc<S>,
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
    A: GetAccountWorkflow,
    S: StatsRepository,
> FinalizeGameWorkflowImpl<G, R, RP, GH, M, NP, SPS, L, A, S>
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
        get_account_workflow: Arc<A>,
        stats_repository: Arc<S>,
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
            get_account_workflow,
            stats_repository,
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
    A: GetAccountWorkflow + Send + Sync + 'static,
    S: StatsRepository + Send + Sync + 'static,
> FinalizeGameWorkflow for FinalizeGameWorkflowImpl<G, R, RP, GH, M, NP, SPS, L, A, S>
{
    async fn finalize_game(&self, ended_game: FinishedGame) {
        let game_id = ended_game.metadata.game_id;
        let over_msg = ListenerMessage::GameOver {
            game_id: game_id,
            game_state: ended_game.game.game_state().clone(),
        };

        let ended_msg = ListenerMessage::GameEnded {
            game: FinishedGameView::from(&ended_game),
        };
        self.listener_notification_port.notify_all(ended_msg);

        self.notify_player_workflow
            .notify_players(
                &[ended_game.metadata.white_id, ended_game.metadata.black_id],
                over_msg.clone(),
            )
            .await;

        let observers = self.spectator_service.get_spectators_for_game(game_id);
        self.listener_notification_port
            .notify_listeners(&observers, over_msg);

        self.spectator_service.remove_game(game_id);

        let game_rating_info = update_ratings(
            &self.get_account_workflow,
            &self.rating_service,
            &self.rating_repository,
            &ended_game,
        )
        .await;

        if let Some(match_id) = self.match_service.get_match_id_by_game_id(game_id) {
            log::info!("Finalizing game {} in match {}", game_id, match_id);
            if !self.match_service.end_game_in_match(match_id, game_id) {
                log::error!("Failed to end game {} in match {}", game_id, match_id);
            }
        } else {
            log::info!("Game {} is not part of a match", game_id);
        }

        update_stats(&self.stats_repository, &ended_game).await;

        let game_record_update = self
            .game_history_service
            .get_finished_game_record_update(ended_game, game_rating_info);
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

async fn update_ratings<
    A: GetAccountWorkflow,
    RS: RatingService + Send + Sync + 'static,
    RR: RatingRepository,
>(
    get_account_workflow: &Arc<A>,
    rating_service: &Arc<RS>,
    rating_repository: &Arc<RR>,
    ended_game: &FinishedGame,
) -> Option<GameRatingInfo> {
    let white_account = get_account_workflow
        .get_account(ended_game.metadata.white_id)
        .await
        .ok();
    let black_account = get_account_workflow
        .get_account(ended_game.metadata.black_id)
        .await
        .ok();

    if white_account.is_none_or(|x| x.is_guest()) || black_account.is_none_or(|x| x.is_guest()) {
        None
    } else {
        let white_id = ended_game.metadata.white_id;
        let black_id = ended_game.metadata.black_id;
        let ended_game_clone = ended_game.clone();
        let rating_service = rating_service.clone();
        match rating_repository
            .update_player_ratings(
                ended_game.metadata.white_id,
                ended_game.metadata.black_id,
                move |w_rating, b_rating| {
                    let mut w_rating = w_rating.unwrap_or(PlayerRating::new(white_id));
                    let mut b_rating = b_rating.unwrap_or(PlayerRating::new(black_id));
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
                    ended_game.metadata.game_id,
                    e
                );
                None
            }
        }
    }
}

async fn update_stats<S: StatsRepository>(stats_repository: &Arc<S>, ended_game: &FinishedGame) {
    let (white_outcome, black_outcome) = match ended_game.game.game_state() {
        TakGameOverState::Draw => (GameOutcome::Draw, GameOutcome::Draw),
        TakGameOverState::Win {
            winner: TakPlayer::White,
            ..
        } => (GameOutcome::Win, GameOutcome::Loss),
        TakGameOverState::Win {
            winner: TakPlayer::Black,
            ..
        } => (GameOutcome::Loss, GameOutcome::Win),
    };

    if let Err(e) = stats_repository
        .update_player_game(
            ended_game.metadata.white_id,
            white_outcome,
            ended_game.metadata.is_rated,
        )
        .await
    {
        log::error!(
            "Failed to update stats for player {}: {}",
            ended_game.metadata.white_id,
            e
        );
    }

    if let Err(e) = stats_repository
        .update_player_game(
            ended_game.metadata.black_id,
            black_outcome,
            ended_game.metadata.is_rated,
        )
        .await
    {
        log::error!(
            "Failed to update stats for player {}: {}",
            ended_game.metadata.black_id,
            e
        );
    }
}
