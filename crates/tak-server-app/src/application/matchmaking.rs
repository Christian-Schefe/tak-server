use std::sync::Arc;

use tak_core::{TakGameSettings, TakPlayer, TakTimeSettings};

use tak_domain::{
    MatchId, SeekId, UserId,
    game::Game,
    game_record::{GameMetadata, GameRecord},
    matches::{Match, MatchMode, MatchSettings, MatchStatus},
    seek::Seek,
};
use terrors::OneOf;

use crate::{
    PortsCollection,
    errors::{
        AppResult, CantBeginGameInMatch, InternalError, InvalidGameSettings, MatchNotFound,
        PlayerNotFound,
    },
    ports::{
        game::{GameRepository, GameStateRepository},
        matches::{MatchReadinessService, MatchRepository},
        notification::{ListenerMatchEventType, ListenerMessage, ListenerNotificationPort},
        seeks::SeekService,
        transaction::TransactionManager,
    },
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::{get_snapshot::GetSnapshotWorkflow, notify_player::NotifyPlayerWorkflow},
};

#[derive(Clone, Debug)]
pub struct SeekView {
    pub id: SeekId,
    pub creator_id: UserId,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub is_rated: bool,
}

impl SeekView {
    pub fn from(id: SeekId, seek: &Seek) -> Self {
        SeekView {
            id,
            creator_id: seek.creator_id,
            color: seek.color,
            game_settings: seek.game_settings.clone(),
            is_rated: seek.is_rated,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchReadinessStatus {
    pub player_ready: Option<UserId>,
}

#[async_trait::async_trait]
pub trait MatchmakingUseCase {
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView>;
    fn list_seeks(&self) -> Vec<SeekView>;
    fn create_seek(
        &self,
        player: UserId,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> AppResult<SeekView, (InvalidGameSettings,)>;
    async fn get_match(
        &self,
        match_id: MatchId,
    ) -> AppResult<Match, (MatchNotFound, InternalError)>;
    fn get_readiness_status(&self, match_id: MatchId) -> MatchReadinessStatus;
    async fn set_player_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> AppResult<(), (MatchNotFound, InternalError)>;
    async fn set_player_not_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> AppResult<(), (MatchNotFound, InternalError)>;
    async fn accept_seek(&self, player: UserId, seek_id: SeekId) -> AppResult<(), AcceptSeekError>;
    fn cancel_seeks(&self, player: UserId);
    fn cancel_seek(&self, player: UserId, seek_id: SeekId) -> bool;
}

pub struct MatchmakingUseCaseImpl<
    Ports: PortsCollection,
    S: SeekService,
    N: ListenerNotificationPort,
    M: MatchReadinessService,
    P: NotifyPlayerWorkflow,
    GT: GameTimeoutRunner,
    GS: GetSnapshotWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    seek_service: Arc<S>,
    notification_port: Arc<N>,
    match_readiness_service: Arc<M>,
    notify_players_workflow: Arc<P>,
    game_timeout_runner: Arc<GT>,
    get_snapshot_workflow: Arc<GS>,
}

impl<
    Ports: PortsCollection,
    S: SeekService,
    N: ListenerNotificationPort,
    M: MatchReadinessService,
    P: NotifyPlayerWorkflow,
    GT: GameTimeoutRunner,
    GS: GetSnapshotWorkflow<Ports::TrCtx>,
> MatchmakingUseCaseImpl<Ports, S, N, M, P, GT, GS>
{
    pub fn new(
        ports: Arc<Ports>,
        seek_service: Arc<S>,
        notification_port: Arc<N>,
        match_readiness_service: Arc<M>,
        notify_players_workflow: Arc<P>,
        game_timeout_runner: Arc<GT>,
        get_snapshot_workflow: Arc<GS>,
    ) -> Self {
        Self {
            ports,
            seek_service,
            notification_port,
            match_readiness_service,
            notify_players_workflow,
            game_timeout_runner,
            get_snapshot_workflow,
        }
    }

    async fn get_match(
        &self,
        match_id: MatchId,
    ) -> AppResult<Match, (MatchNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let Some(match_entry) = self
            .ports
            .match_repository()
            .get_match(&ctx, match_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(MatchNotFound));
        };
        Ok(match_entry)
    }

    #[tracing::instrument(skip(self, ctx))]
    async fn create_game_from_match(
        &self,
        ctx: &Ports::TrCtx,
        match_id: MatchId,
    ) -> AppResult<
        (),
        (
            CantBeginGameInMatch,
            MatchNotFound,
            PlayerNotFound,
            InternalError,
        ),
    > {
        let date = chrono::Utc::now();

        let Some(mut match_entry) = self
            .ports
            .match_repository()
            .get_match(ctx, match_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(MatchNotFound));
        };
        let player1_color = match_entry.try_begin_game().map_err(|e| {
            tracing::error!("Failed to start game for match {}: {}", match_id, e);
            OneOf::new(CantBeginGameInMatch)
        })?;

        let (white_id, black_id) = match player1_color {
            TakPlayer::White => (match_entry.player1.user_id, match_entry.player2.user_id),
            TakPlayer::Black => (match_entry.player2.user_id, match_entry.player1.user_id),
        };

        let Some(snapshot_white) = self
            .get_snapshot_workflow
            .get_snapshot(ctx, white_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(PlayerNotFound));
        };
        let Some(snapshot_black) = self
            .get_snapshot_workflow
            .get_snapshot(ctx, black_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(PlayerNotFound));
        };

        let metadata = GameMetadata::new(
            date,
            snapshot_white,
            snapshot_black,
            match_entry.settings.is_rated,
            match_entry.settings.game_settings.clone(),
            Some(match_id),
        );

        let game_state = Game::new(&match_entry.settings.game_settings).to_game_state();

        let game_record = GameRecord::new(metadata);

        let game_id = self
            .ports
            .game_repository()
            .create_game(ctx, game_record.clone())
            .await
            .map_err(OneOf::broaden)?;
        self.ports
            .game_state_repository()
            .set_state(ctx, game_id, game_state)
            .await
            .map_err(OneOf::broaden)?;

        self.ports
            .match_repository()
            .update_match(ctx, match_id, match_entry)
            .await
            .map_err(OneOf::broaden)?;
        GameTimeoutRunner::schedule_game_timeout_check(self.game_timeout_runner.clone(), game_id);
        let msg = ListenerMessage::GameStarted {
            game_id,
            game: game_record,
        };
        self.notification_port.notify_all(&msg);

        Ok(())
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    S: SeekService + Send + Sync + 'static,
    N: ListenerNotificationPort + Send + Sync + 'static,
    M: MatchReadinessService + Send + Sync + 'static,
    P: NotifyPlayerWorkflow + Send + Sync + 'static,
    GT: GameTimeoutRunner + Send + Sync + 'static,
    GS: GetSnapshotWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> MatchmakingUseCase for MatchmakingUseCaseImpl<Ports, S, N, M, P, GT, GS>
{
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView> {
        self.seek_service
            .get_seek(seek_id)
            .map(|seek| SeekView::from(seek_id, &seek))
    }

    fn list_seeks(&self) -> Vec<SeekView> {
        let res = self
            .seek_service
            .list_seeks()
            .into_iter()
            .map(|(seek_id, seek)| SeekView::from(seek_id, &seek))
            .collect();
        res
    }

    fn create_seek(
        &self,
        player: UserId,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> AppResult<SeekView, (InvalidGameSettings,)> {
        let Some(seek) = Seek::new(player, color, game_settings, is_rated) else {
            return Err(OneOf::new(InvalidGameSettings));
        };
        let seek_id = self.seek_service.create_seek(seek.clone());
        let seek_view: SeekView = SeekView::from(seek_id, &seek);
        let message = ListenerMessage::SeekCreated {
            seek: seek_view.clone(),
        };

        self.notification_port.notify_all(&message);

        Ok(seek_view)
    }

    async fn get_match(
        &self,
        match_id: MatchId,
    ) -> AppResult<Match, (MatchNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let Some(m) = self
            .ports
            .match_repository()
            .get_match(&ctx, match_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(MatchNotFound));
        };
        Ok(m)
    }

    fn get_readiness_status(&self, match_id: MatchId) -> MatchReadinessStatus {
        MatchReadinessStatus {
            player_ready: self.match_readiness_service.get_readiness_status(match_id),
        }
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn set_player_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> AppResult<(), (MatchNotFound, InternalError)> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1.user_id != user_id && match_entry.player2.user_id != user_id {
            tracing::error!(
                "Player {} is not a participant in match {}",
                user_id,
                match_id
            );
            return Err(OneOf::new(MatchNotFound));
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows readiness. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(OneOf::new(MatchNotFound));
        };
        let should_create_game = self
            .match_readiness_service
            .set_player_ready(match_id, user_id);

        if should_create_game {
            let ctx = self
                .ports
                .transaction_manager()
                .transaction()
                .await
                .map_err(OneOf::broaden)?;
            if let Err(e) = self.create_game_from_match(&ctx, match_id).await {
                tracing::error!("Failed to create game from match {}: {:?}", match_id, e);
                return Err(OneOf::new(InternalError));
            }
            self.ports
                .transaction_manager()
                .commit(ctx)
                .await
                .map_err(OneOf::broaden)?;
        } else {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchReadinessChanged {
                    user_id: Some(user_id),
                },
            };

            self.notify_players_workflow.notify_players(
                &[match_entry.player1.user_id, match_entry.player2.user_id],
                &msg,
            );
        }

        Ok(())
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn set_player_not_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> Result<(), MatchReadinessError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1.user_id != user_id && match_entry.player2.user_id != user_id {
            tracing::error!(
                "Player {} is not a participant in match {}",
                user_id,
                match_id
            );
            return Err(MatchReadinessError::MatchNotFound);
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows retracting readiness. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(MatchReadinessError::Internal);
        };
        let did_remove = self
            .match_readiness_service
            .set_player_not_ready(match_id, user_id);
        if did_remove {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchReadinessChanged { user_id: None },
            };
            self.notify_players_workflow.notify_players(
                &[match_entry.player1.user_id, match_entry.player2.user_id],
                &msg,
            );
        }
        Ok(())
    }
    #[tracing::instrument(skip(self))]
    async fn accept_seek(&self, player: UserId, seek_id: SeekId) -> Result<(), AcceptSeekError> {
        let seek = self
            .seek_service
            .remove_seek_if(seek_id, |s| s.creator_id != player)
            .ok_or(AcceptSeekError::SeekNotFound)?;

        let cancelled_seeks = self.seek_service.cancel_player_seeks(player, |seek| {
            matches!(
                seek.game_settings.time_settings,
                TakTimeSettings::Realtime(_)
            )
        });
        let cancelled_seeks2 = self
            .seek_service
            .cancel_player_seeks(seek.creator_id, |seek| {
                matches!(
                    seek.game_settings.time_settings,
                    TakTimeSettings::Realtime(_)
                )
            });

        for (cancelled_seek_id, cancelled_seek) in cancelled_seeks
            .into_iter()
            .chain(cancelled_seeks2.into_iter())
        {
            let message = ListenerMessage::SeekCancelled {
                seek: SeekView::from(cancelled_seek_id, &cancelled_seek),
            };
            self.notification_port.notify_all(&message);
        }

        let message = ListenerMessage::SeekAccepted {
            seek: SeekView::from(seek_id, &seek),
        };
        self.notification_port.notify_all(&message);

        let initial_color = seek.color.unwrap_or_else(|| {
            if rand::random::<bool>() {
                TakPlayer::White
            } else {
                TakPlayer::Black
            }
        });

        let match_settings = MatchSettings {
            game_settings: seek.game_settings.clone(),
            match_mode: MatchMode::Unlimited,
            is_rated: seek.is_rated,
        };

        let match_data = Match::new(
            seek.creator_id,
            player,
            None,
            match_settings.clone(),
            initial_color,
        );

        let ctx = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to start transaction for accepting seek {}: {}",
                    seek_id,
                    e
                );
                AcceptSeekError::Internal
            })?;
        let match_id = match self
            .ports
            .match_repository()
            .create_match(&ctx, match_data)
            .await
        {
            Ok(id) => id,
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to create match from accepted seek {}: {}",
                    seek_id,
                    e
                );
                return Err(AcceptSeekError::Internal);
            }
        };

        match self.create_game_from_match(&ctx, match_id).await {
            Ok(_) => Ok(()),
            Err(CreateGameFromMatchError::PlayerNotFound) => {
                tracing::error!(
                    "Failed to create game from match {}: player not found",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
            Err(CreateGameFromMatchError::AlreadyInProgress) => {
                tracing::error!(
                    "Failed to create game from match {}: already in progress",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
            Err(CreateGameFromMatchError::Internal) => {
                tracing::error!(
                    "Failed to create game from match {}: internal error",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
            Err(CreateGameFromMatchError::MatchNotFound) => {
                tracing::error!(
                    "Failed to create game from match {}: match not found",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
        }?;

        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to commit transaction for accepting seek {}: {}",
                    seek_id,
                    e
                );
                AcceptSeekError::Internal
            })?;
        Ok(())
    }

    fn cancel_seeks(&self, player: UserId) {
        for (cancelled_seek_id, cancelled_seek) in
            self.seek_service.cancel_player_seeks(player, |_| true)
        {
            let message = ListenerMessage::SeekCancelled {
                seek: SeekView::from(cancelled_seek_id, &cancelled_seek),
            };
            self.notification_port.notify_all(&message);
        }
    }

    fn cancel_seek(&self, player: UserId, seek_id: SeekId) -> bool {
        if let Some(cancelled_seek) = self.seek_service.cancel_seek(player, seek_id) {
            let message = ListenerMessage::SeekCancelled {
                seek: SeekView::from(seek_id, &cancelled_seek),
            };
            self.notification_port.notify_all(&message);
            return true;
        }
        false
    }
}
