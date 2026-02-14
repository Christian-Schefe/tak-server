use tak_core::{TakAction, TakGameResult, TakTimeInfo, TakTimeSettings};

use crate::{
    domain::{
        GameId,
        game::{GameEvent, GameEventType},
        game_history::{GameRatingInfo, GameRecord, PlayerSnapshot},
    },
    workflow::gameplay::GameMetadataView,
};

pub mod query;

pub struct GameRecordView {
    pub game_id: GameId,
    pub metadata: GameMetadataView,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub rating_info: Option<GameRatingInfo>,
    pub result: Option<TakGameResult>,
    pub events: Vec<GameEvent>,
}

impl GameRecordView {
    pub fn from_game_record(game_id: GameId, record: GameRecord) -> Self {
        Self {
            game_id,
            metadata: GameMetadataView::from(&record.metadata),
            white: record.white,
            black: record.black,
            rating_info: record.rating_info,
            result: record.result,
            events: record.events,
        }
    }

    pub fn reconstruct_action_history(&self) -> Vec<TakAction> {
        let mut actions = Vec::new();
        for event in &self.events {
            if let GameEventType::Action { action, .. } = &event.event_type {
                actions.push(action.clone());
            } else if let GameEventType::ActionUndone { .. } = &event.event_type {
                actions.pop();
            }
        }
        actions
    }

    pub fn reconstruct_time_info(&self) -> TakTimeInfo {
        let mut maybe_time_info = None;
        for event in &self.events {
            match &event.event_type {
                GameEventType::Action { time_info, .. } => {
                    maybe_time_info = Some(time_info);
                }
                _ => {}
            }
        }

        match maybe_time_info {
            Some(ti) => ti.clone(),
            None => match &self.metadata.settings.time_settings {
                TakTimeSettings::Realtime(s) => TakTimeInfo {
                    white_remaining: s.contingent,
                    black_remaining: s.contingent,
                },
                TakTimeSettings::Async(s) => TakTimeInfo {
                    white_remaining: s.contingent,
                    black_remaining: s.contingent,
                },
            },
        }
    }
}
