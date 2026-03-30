use std::collections::HashMap;

use tak_core::TakGameSettings;

use crate::domain::{
    PlayerId, TournamentId,
    tournament::{Tournament, TournamentFormat, TournamentMetadata, TournamentPlayer},
};

pub mod get;
pub mod host;
pub mod register;
pub mod tournament_match;

#[derive(Clone, Debug)]
pub struct TournamentMetadataView {
    pub tournament_id: TournamentId,
    pub name: String,
    pub tournament_format: TournamentFormat,
    pub match_settings: TakGameSettings,
}

#[derive(Clone, Debug)]
pub struct TournamentView {
    pub metadata: TournamentMetadataView,
}

#[derive(Clone, Debug)]
pub struct TournamentDetailView {
    pub metadata: TournamentMetadataView,
    pub player_scores: HashMap<PlayerId, u32>,
}

impl TournamentMetadataView {
    pub fn from_metadata(tournament_id: TournamentId, metadata: TournamentMetadata) -> Self {
        Self {
            tournament_id,
            name: metadata.name,
            tournament_format: metadata.tournament_format,
            match_settings: metadata.match_settings,
        }
    }
}

impl TournamentView {
    pub fn from_tournament(tournament_id: TournamentId, tournament: Tournament) -> Self {
        Self {
            metadata: TournamentMetadataView::from_metadata(tournament_id, tournament.metadata),
        }
    }
}

impl TournamentDetailView {
    pub fn from_tournament(
        tournament_id: TournamentId,
        tournament: Tournament,
        tournament_players: Vec<TournamentPlayer>,
    ) -> Self {
        Self {
            metadata: TournamentMetadataView::from_metadata(tournament_id, tournament.metadata),
            player_scores: tournament_players
                .into_iter()
                .map(|tp| (tp.player_id, tp.score))
                .collect(),
        }
    }
}
