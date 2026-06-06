use std::collections::HashMap;

use tak_core::TakGameSettings;

use crate::domain::{
    MatchId, PlayerId, TournamentId,
    tournament::{
        Tournament, TournamentFormat, TournamentMetadata, TournamentPlayer, TournamentRound,
        TournamentStatus,
    },
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
    pub status: TournamentStatus,
}

#[derive(Clone, Debug)]
pub struct TournamentDetailView {
    pub tournament: TournamentView,
    pub player_half_scores: HashMap<PlayerId, u32>,
    pub rounds: Vec<TournamentRoundView>,
}

#[derive(Clone, Debug)]
pub struct TournamentRoundView {
    pub matches: Vec<MatchId>,
    pub byes: Vec<PlayerId>,
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
            status: tournament.status,
        }
    }
}

impl TournamentDetailView {
    pub fn from_tournament(
        tournament_id: TournamentId,
        tournament: Tournament,
        tournament_players: Vec<TournamentPlayer>,
        rounds: Vec<TournamentRound>,
    ) -> Self {
        Self {
            tournament: TournamentView::from_tournament(tournament_id, tournament),
            player_half_scores: tournament_players
                .into_iter()
                .map(|tp| (tp.player_id, tp.half_score))
                .collect(),
            rounds: rounds
                .into_iter()
                .map(TournamentRoundView::from_tournament_round)
                .collect(),
        }
    }
}

impl TournamentRoundView {
    pub fn from_tournament_round(tournament_round: TournamentRound) -> Self {
        Self {
            matches: tournament_round.matches,
            byes: tournament_round.byes,
        }
    }
}
