use tak_core::TakGameSettings;

use crate::domain::{
    PlayerId, TournamentId,
    tournament::{Tournament, TournamentMetadata, TournamentType},
};

pub mod get;
pub mod host;
pub mod register;

#[derive(Clone, Debug)]
pub struct TournamentMetadataView {
    pub tournament_id: TournamentId,
    pub name: String,
    pub tournament_type: TournamentType,
    pub match_settings: TakGameSettings,
}

#[derive(Clone, Debug)]
pub struct TournamentView {
    pub metadata: TournamentMetadataView,
}

#[derive(Clone, Debug)]
pub struct TournamentDetailView {
    pub metadata: TournamentMetadataView,
    pub registered_players: Vec<PlayerId>,
}

impl TournamentMetadataView {
    pub fn from_metadata(tournament_id: TournamentId, metadata: TournamentMetadata) -> Self {
        Self {
            tournament_id,
            name: metadata.name,
            tournament_type: metadata.tournament_type,
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
        registered_players: Vec<PlayerId>,
    ) -> Self {
        Self {
            metadata: TournamentMetadataView::from_metadata(tournament_id, tournament.metadata),
            registered_players,
        }
    }
}
