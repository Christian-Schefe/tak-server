use std::sync::Arc;

use tak_core::{TakGameSettings, TakPlayer};

use better_errors::OneOf;
use tak_domain::{
    MatchId, SeekId, UserId,
    game::Game,
    game_record::{GameMetadata, GameRecord},
    matches::{Match, MatchMode, MatchSettings, MatchStatus},
    seek::Seek,
};

use crate::{
    PortsCollection,
    errors::{
        AppResult, InternalError, InvalidGameSettings, InvalidMatchStatus, MatchNotFound,
        SeekNotFound, UnexpectedPlayer, UserNotFound,
    },
    ports::notification::{NotificationMatchEventType, NotificationMessage},
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::get_snapshot::get_snapshot,
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
    ) -> AppResult<
        (),
        (
            UnexpectedPlayer,
            InvalidMatchStatus,
            MatchNotFound,
            InternalError,
        ),
    >;
    async fn accept_seek(
        &self,
        player: UserId,
        seek_id: SeekId,
    ) -> AppResult<(), (SeekNotFound, InternalError)>;
    fn cancel_seeks(&self, player: UserId);
    fn cancel_seek(&self, player: UserId, seek_id: SeekId) -> bool;
}

pub struct MatchmakingUseCaseImpl<Ports: PortsCollection, GT: GameTimeoutRunner> {
    ports: Arc<Ports>,
    game_timeout_runner: Arc<GT>,
}

impl<Ports: PortsCollection, GT: GameTimeoutRunner> MatchmakingUseCaseImpl<Ports, GT> {
    pub fn new(ports: Arc<Ports>, game_timeout_runner: Arc<GT>) -> Self {
        Self {
            ports,
            game_timeout_runner,
        }
    }
}

impl<Ports: PortsCollection + Send + Sync + 'static, GT: GameTimeoutRunner + Send + Sync + 'static>
    MatchmakingUseCaseImpl<Ports, GT>
{
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
        ctx: &Ports::RpCtx,
        match_id: MatchId,
    ) -> AppResult<
        (),
        (
            InvalidMatchStatus,
            MatchNotFound,
            UserNotFound,
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
            OneOf::new(InvalidMatchStatus)
        })?;

        let (white_id, black_id) = match player1_color {
            TakPlayer::White => (match_entry.player1.user_id, match_entry.player2.user_id),
            TakPlayer::Black => (match_entry.player2.user_id, match_entry.player1.user_id),
        };

        let Some(snapshot_white) = get_snapshot(self.ports.as_ref(), ctx, white_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
        };
        let Some(snapshot_black) = get_snapshot(self.ports.as_ref(), ctx, black_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
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
        let msg = NotificationMessage::GameStarted {
            game_id,
            game: game_record,
        };
        self.ports.notification_port().notify_all(&msg);

        Ok(())
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static, GT: GameTimeoutRunner + Send + Sync + 'static>
    MatchmakingUseCase for MatchmakingUseCaseImpl<Ports, GT>
{
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView> {
        self.ports
            .seek_service()
            .get_seek(seek_id)
            .map(|seek| SeekView::from(seek_id, &seek))
    }

    fn list_seeks(&self) -> Vec<SeekView> {
        let res = self
            .ports
            .seek_service()
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
        let seek_id = self.ports.seek_service().create_seek(seek.clone());
        let seek_view: SeekView = SeekView::from(seek_id, &seek);
        let message = NotificationMessage::SeekCreated {
            seek: seek_view.clone(),
        };

        self.ports.notification_port().notify_all(&message);

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
            player_ready: self
                .ports
                .match_readiness_service()
                .get_readiness_status(match_id),
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
            .ports
            .match_readiness_service()
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
            let msg = NotificationMessage::MatchEvent {
                match_id,
                event_type: NotificationMatchEventType::MatchReadinessChanged {
                    user_id: Some(user_id),
                },
            };

            self.ports.notification_port().notify_users(
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
    ) -> AppResult<
        (),
        (
            UnexpectedPlayer,
            InvalidMatchStatus,
            MatchNotFound,
            InternalError,
        ),
    > {
        let match_entry = self.get_match(match_id).await.map_err(OneOf::broaden)?;
        if match_entry.player1.user_id != user_id && match_entry.player2.user_id != user_id {
            tracing::error!(
                "Player {} is not a participant in match {}",
                user_id,
                match_id
            );
            return Err(OneOf::new(UnexpectedPlayer));
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows retracting readiness. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(OneOf::new(InvalidMatchStatus));
        };
        let did_remove = self
            .ports
            .match_readiness_service()
            .set_player_not_ready(match_id, user_id);
        if did_remove {
            let msg = NotificationMessage::MatchEvent {
                match_id,
                event_type: NotificationMatchEventType::MatchReadinessChanged { user_id: None },
            };
            self.ports.notification_port().notify_users(
                &[match_entry.player1.user_id, match_entry.player2.user_id],
                &msg,
            );
        }
        Ok(())
    }
    #[tracing::instrument(skip(self))]
    async fn accept_seek(
        &self,
        player: UserId,
        seek_id: SeekId,
    ) -> AppResult<(), (SeekNotFound, InternalError)> {
        let seek = self
            .ports
            .seek_service()
            .try_accept_seek(seek_id, player)
            .ok_or(OneOf::new(SeekNotFound))?;

        let cancelled_seeks = self.ports.seek_service().cancel_player_seeks(player, true);
        let cancelled_seeks2 = self
            .ports
            .seek_service()
            .cancel_player_seeks(seek.creator_id, true);

        for (cancelled_seek_id, cancelled_seek) in cancelled_seeks
            .into_iter()
            .chain(cancelled_seeks2.into_iter())
        {
            let message = NotificationMessage::SeekCancelled {
                seek: SeekView::from(cancelled_seek_id, &cancelled_seek),
            };
            self.ports.notification_port().notify_all(&message);
        }

        let message = NotificationMessage::SeekAccepted {
            seek: SeekView::from(seek_id, &seek),
        };
        self.ports.notification_port().notify_all(&message);

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
            .map_err(OneOf::broaden)?;
        let match_id = self
            .ports
            .match_repository()
            .create_match(&ctx, match_data)
            .await
            .map_err(OneOf::broaden)?;

        match self.create_game_from_match(&ctx, match_id).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to create game from match {}: {:?}", match_id, e);
                return Err(OneOf::new(InternalError));
            }
        }

        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    fn cancel_seeks(&self, player: UserId) {
        for (cancelled_seek_id, cancelled_seek) in
            self.ports.seek_service().cancel_player_seeks(player, false)
        {
            let message = NotificationMessage::SeekCancelled {
                seek: SeekView::from(cancelled_seek_id, &cancelled_seek),
            };
            self.ports.notification_port().notify_all(&message);
        }
    }

    fn cancel_seek(&self, player: UserId, seek_id: SeekId) -> bool {
        if let Some(cancelled_seek) = self.ports.seek_service().cancel_seek(player, seek_id) {
            let message = NotificationMessage::SeekCancelled {
                seek: SeekView::from(seek_id, &cancelled_seek),
            };
            self.ports.notification_port().notify_all(&message);
            return true;
        }
        false
    }
}
