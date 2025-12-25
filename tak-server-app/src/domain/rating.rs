use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tak_core::{TakActionRecord, TakGameSettings, TakGameState, TakPlayer};

use crate::domain::{GameType, PlayerId, game::Game, game_history::GameRatingInfo};

#[derive(Clone, Debug)]
pub struct PlayerRating {
    pub rating: f64,
    pub boost: f64,
    pub max_rating: f64,
    pub rated_games_played: u32,
    pub is_unrated: bool,
    pub participation_rating: f64,
    pub rating_age: f64,
    pub fatigue: HashMap<PlayerId, f64>,
}

impl PlayerRating {
    pub fn new() -> Self {
        Self {
            rating: 1000.0,
            boost: 750.0,
            max_rating: 1000.0,
            rated_games_played: 0,
            is_unrated: false,
            participation_rating: 0.0,
            rating_age: 0.0,
            fatigue: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
pub trait RatingRepository {
    async fn get_player_rating(&self, player_id: PlayerId) -> PlayerRating;
    async fn update_player_ratings<R>(
        &self,
        white: PlayerId,
        black: PlayerId,
        calc_fn: impl FnOnce(&mut PlayerRating, &mut PlayerRating) -> R + Send,
    ) -> R;
}

pub trait RatingService {
    fn get_current_rating(&self, player_rating: &PlayerRating, date: DateTime<Utc>) -> f64;
    fn calculate_ratings(
        &self,
        date: DateTime<Utc>,
        game: &Game,
        white_rating: &mut PlayerRating,
        black_rating: &mut PlayerRating,
    ) -> Option<GameRatingInfo>;
}

pub struct RatingServiceImpl;

impl RatingServiceImpl {
    const INITIAL_RATING: f64 = 1000.0;
    const BONUS_RATING: f64 = 750.0;
    const BONUS_FACTOR: f64 = 60.0;
    const PARTICIPATION_LIMIT: f64 = 10.0;
    const PARTICIPATION_CUTOFF: f64 = 1500.0;
    const MAX_DROP: f64 = 200.0;
    const RATING_RETENTION: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 240.0;

    pub fn new() -> Self {
        Self {}
    }

    fn is_game_eligible_for_rating(
        &self,
        settings: &TakGameSettings,
        game_type: GameType,
        result: &TakGameState,
        moves: &Vec<TakActionRecord>,
    ) -> bool {
        match game_type {
            GameType::Unrated => return false,
            _ => {}
        };
        if settings.board_size < 5 {
            return false;
        }
        const TIME_LIMITS: [u32; 4] = [180, 240, 300, 360];
        const PIECE_LIMITS: [(u32, u32); 4] = [(20, 32), (25, 40), (30, 48), (40, 64)];
        const CAPSTONE_LIMITS: [(u32, u32); 4] = [(1, 1), (1, 2), (1, 2), (1, 3)];

        let size_index = ((settings.board_size - 5) as usize).min(3);

        let contingent_secs = settings.time_control.contingent.as_secs();
        let time_score = contingent_secs * 3 + settings.time_control.increment.as_secs();
        if time_score < TIME_LIMITS[size_index] as u64 || contingent_secs < 60 {
            return false;
        }
        let reserve = &settings.reserve;
        if reserve.pieces < PIECE_LIMITS[size_index].0
            || reserve.pieces > PIECE_LIMITS[size_index].1
        {
            return false;
        }
        if reserve.capstones < CAPSTONE_LIMITS[size_index].0
            || reserve.capstones > CAPSTONE_LIMITS[size_index].1
        {
            return false;
        }

        match result {
            TakGameState::Ongoing => return false,
            _ => {}
        }

        if moves.len() <= 6 {
            return false;
        }

        true
    }

    fn calc_decayed_rating(
        rating: &PlayerRating,
        date: i64,
        participation_cutoff: f64,
        rating_retention: f64,
        max_drop: f64,
        participation_limit: f64,
    ) -> f64 {
        if rating.rating < participation_cutoff {
            return rating.rating;
        }
        let participation = (20.0
            * (0.5f64).powf((date as f64 - rating.rating_age) / rating_retention))
            / participation_limit;

        if rating.rating < participation_cutoff + max_drop {
            rating
                .rating
                .min(participation_cutoff + (max_drop * participation))
        } else {
            rating.rating - (max_drop * (1.0 - participation).max(0.0))
        }
    }

    fn update_rating(
        player: &mut PlayerRating,
        amount: f64,
        fairness: f64,
        fatigue_factor: f64,
        date: f64,
        bonus_factor: f64,
        bonus_rating: f64,
        rating_retention: f64,
        initial_rating: f64,
    ) {
        let bonus = f64::min(
            f64::max(
                0.0,
                (fatigue_factor * amount * f64::max(player.boost, 1.0) * bonus_factor)
                    / bonus_rating,
            ),
            player.boost,
        );
        player.boost -= bonus;
        let k = 10.0
            + 15.0 * (0.5f64).powf(player.rated_games_played as f64 / 200.0)
            + 15.0 * (0.5f64).powf((player.max_rating - initial_rating) / 300.0);
        player.rating += fatigue_factor * amount * k + bonus;
        if player.rating_age == 0.0 {
            player.rating_age = date - rating_retention;
        }
        let participation = f64::min(
            20.0,
            20.0 * (0.5f64).powf((date - player.rating_age) / rating_retention)
                + fairness * fatigue_factor,
        );
        player.rating_age = f64::log2(participation / 20.0) * rating_retention + date;
        player.rated_games_played += 1;
        player.max_rating = f64::max(player.max_rating, player.rating);
    }

    fn update_fatigue(player: &mut PlayerRating, opponent_id: PlayerId, game_factor: f64) {
        let multiplier = 1.0 - game_factor * 0.4;
        for (_, fatigue) in player.fatigue.iter_mut() {
            *fatigue *= multiplier;
        }
        player.fatigue.retain(|&id, &mut f| {
            if id != opponent_id && f < 0.01 {
                false
            } else {
                true
            }
        });
        player
            .fatigue
            .entry(opponent_id)
            .and_modify(|f| *f += game_factor)
            .or_insert(game_factor);
    }

    fn update_rating_and_fatigue(
        player: &mut PlayerRating,
        opponent_id: PlayerId,
        amount: f64,
        fairness: f64,
        fatigue_factor: f64,
        date: DateTime<Utc>,
    ) {
        Self::update_rating(
            player,
            amount,
            fairness,
            fatigue_factor,
            date.timestamp() as f64,
            Self::BONUS_FACTOR,
            Self::BONUS_RATING,
            Self::RATING_RETENTION,
            Self::INITIAL_RATING,
        );
        Self::update_fatigue(player, opponent_id, fairness * fatigue_factor);
    }
}

impl RatingService for RatingServiceImpl {
    fn get_current_rating(&self, player_rating: &PlayerRating, date: DateTime<Utc>) -> f64 {
        Self::calc_decayed_rating(
            player_rating,
            date.timestamp(),
            Self::PARTICIPATION_CUTOFF,
            Self::RATING_RETENTION,
            Self::MAX_DROP,
            Self::PARTICIPATION_LIMIT,
        )
    }

    fn calculate_ratings(
        &self,
        date: DateTime<Utc>,
        game: &Game,
        white_rating: &mut PlayerRating,
        black_rating: &mut PlayerRating,
    ) -> Option<GameRatingInfo> {
        let white_id = game.white;
        let black_id = game.black;
        let result = game.game.game_state();

        if !self.is_game_eligible_for_rating(
            &game.settings,
            game.game_type,
            &result,
            game.game.action_history(),
        ) {
            return None;
        }

        let result = match &result {
            TakGameState::Win { winner, .. } => match winner {
                TakPlayer::White => 1.0,
                TakPlayer::Black => 0.0,
            },
            TakGameState::Draw => 0.5,
            _ => return None,
        };

        let old_white_rating_decayed = self.get_current_rating(&white_rating, date);
        let old_black_rating_decayed = self.get_current_rating(&black_rating, date);

        let sw = 10f64.powf(white_rating.rating / 400.0);
        let sb = 10f64.powf(black_rating.rating / 400.0);
        let expected = sw / (sw + sb);
        let fairness = expected * (1.0 - expected);
        let fatigue_factor = (1.0 - white_rating.fatigue.get(&black_id).unwrap_or(&0.0) * 0.4)
            * (1.0 - black_rating.fatigue.get(&white_id).unwrap_or(&0.0) * 0.4);
        let adjustment = result - expected;
        Self::update_rating_and_fatigue(
            white_rating,
            black_id,
            adjustment,
            fairness,
            fatigue_factor,
            date,
        );
        Self::update_rating_and_fatigue(
            black_rating,
            white_id,
            -adjustment,
            fairness,
            fatigue_factor,
            date,
        );

        let new_white_rating_decayed = self.get_current_rating(&white_rating, date);
        let new_black_rating_decayed = self.get_current_rating(&black_rating, date);

        Some(GameRatingInfo {
            rating_change_white: new_white_rating_decayed - old_white_rating_decayed,
            rating_change_black: new_black_rating_decayed - old_black_rating_decayed,
        })
    }
}
