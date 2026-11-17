use tak_core::{TakGameSettings, TakPlayer};

use crate::{TournamentId, UserId};

#[derive(Clone, Debug)]
pub struct Match {
    pub settings: MatchSettings,
    pub player1: MatchPlayer,
    pub player2: MatchPlayer,
    pub status: MatchStatus,
    pub games_played: u32,
    pub tournament_info: Option<MatchTournamentInfo>,
    pub initial_color: TakPlayer,
}

#[derive(Clone, Debug)]
pub struct MatchPlayer {
    pub user_id: UserId,
    pub score: u32,
}

#[derive(Clone, Debug)]
pub struct MatchSettings {
    pub game_settings: TakGameSettings,
    pub match_mode: MatchMode,
    pub is_rated: bool,
    // TODO: tiebreak: Option<TiebreakSettings>,
}

#[derive(Clone, Debug)]
pub struct MatchTournamentInfo {
    pub tournament_id: TournamentId,
    pub round: u32,
    pub round_match_number: u32,
}

#[derive(Clone, Debug)]
pub enum MatchMode {
    Unlimited,
    FixedGames(u32),
    FirstTo(u32),
}

impl Match {
    pub fn new(
        player1: UserId,
        player2: UserId,
        tournament_info: Option<MatchTournamentInfo>,
        settings: MatchSettings,
        initial_color: TakPlayer,
    ) -> Self {
        Self {
            player1: MatchPlayer {
                user_id: player1,
                score: 0,
            },
            player2: MatchPlayer {
                user_id: player2,
                score: 0,
            },
            settings,
            status: MatchStatus::Waiting,
            initial_color,
            games_played: 0,
            tournament_info,
        }
    }

    pub fn get_winner(&self) -> Option<UserId> {
        match self.status {
            MatchStatus::Completed => {
                if self.player1.score > self.player2.score {
                    Some(self.player1.user_id)
                } else if self.player2.score > self.player1.score {
                    Some(self.player2.user_id)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn abort_game_in_match(&mut self) {
        self.status = MatchStatus::Waiting;
    }

    pub fn end_game_in_match(&mut self, winner: Option<bool>) {
        match winner {
            Some(is_player_one) => {
                if is_player_one {
                    self.player1.score += 2;
                } else {
                    self.player2.score += 2;
                }
            }
            None => {
                self.player1.score += 1;
                self.player2.score += 1;
            }
        }
        self.games_played += 1;
        let completed = match self.settings.match_mode {
            MatchMode::Unlimited => false,
            MatchMode::FixedGames(total_games) => self.games_played >= total_games,
            MatchMode::FirstTo(score) => self.player1.score >= score || self.player2.score >= score,
        };
        if completed {
            self.status = MatchStatus::Completed;
        } else {
            self.status = MatchStatus::Waiting;
        }
    }

    pub fn try_begin_game(&mut self) -> Result<TakPlayer, String> {
        match self.status {
            MatchStatus::Waiting => {
                self.status = MatchStatus::Ongoing;
                let player1_color = if self.games_played % 2 == 0 {
                    self.initial_color
                } else {
                    self.initial_color.opponent()
                };
                Ok(player1_color)
            }
            MatchStatus::Ongoing => Err("Game is already in progress".to_string()),
            MatchStatus::Completed => Err("Match is already completed".to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MatchStatus {
    Waiting,
    Ongoing,
    Completed,
}
