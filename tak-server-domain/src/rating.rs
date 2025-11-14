use std::collections::HashMap;

use crate::{
    ServiceResult,
    game::{ArcGameRepository, GameId, GameRecord, GameType},
    player::{ArcPlayerRepository, ArcPlayerService, Player, PlayerId},
};

#[derive(Clone, Debug)]
pub struct PlayerRating {
    pub rating: f64,
    pub boost: f64,
    pub max_rating: f64,
    pub rated_games_played: u32,
    pub unrated_games_played: u32,
    pub participation_rating: f64,
    pub rating_base: f64,
    pub rating_age: f64,
    pub fatigue: HashMap<GameId, f64>,
}

impl PlayerRating {
    pub fn new() -> Self {
        Self {
            rating: 1000.0,
            boost: 750.0,
            max_rating: 1000.0,
            rated_games_played: 0,
            unrated_games_played: 0,
            participation_rating: 0.0,
            rating_base: 1000.0,
            rating_age: 0.0,
            fatigue: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameRatingInfo {
    pub rating_white: f64,
    pub rating_black: f64,
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}

pub struct RatingServiceImpl {
    player_service: ArcPlayerService,
    player_repository: ArcPlayerRepository,
    game_repository: ArcGameRepository,
}

impl RatingServiceImpl {
    pub fn new(
        player_repository: ArcPlayerRepository,
        game_repository: ArcGameRepository,
        player_service: ArcPlayerService,
    ) -> Self {
        Self {
            player_repository,
            game_repository,
            player_service,
        }
    }

    fn is_game_eligible_for_rating(&self, game_record: &GameRecord) -> bool {
        match game_record.game_type {
            GameType::Unrated => return false,
            _ => {}
        };
        if game_record.settings.board_size < 5 {
            return false;
        }
        const TIME_LIMITS: [u32; 4] = [180, 240, 300, 360];
        const PIECE_LIMITS: [(u32, u32); 4] = [(20, 32), (25, 40), (30, 48), (40, 64)];
        const CAPSTONE_LIMITS: [(u32, u32); 4] = [(1, 1), (1, 2), (1, 2), (1, 3)];

        let size_index = ((game_record.settings.board_size - 5) as usize).min(3);

        let contingent_secs = game_record.settings.time_control.contingent.as_secs();
        let time_score =
            contingent_secs * 3 + game_record.settings.time_control.increment.as_secs();
        if time_score < TIME_LIMITS[size_index] as u64 || contingent_secs < 60 {
            return false;
        }
        if game_record.settings.reserve_pieces < PIECE_LIMITS[size_index].0
            || game_record.settings.reserve_pieces > PIECE_LIMITS[size_index].1
        {
            return false;
        }
        if game_record.settings.reserve_capstones < CAPSTONE_LIMITS[size_index].0
            || game_record.settings.reserve_capstones > CAPSTONE_LIMITS[size_index].1
        {
            return false;
        }
        true
    }

    fn adjusted_rating(
        player: Player,
        date: i64,
        participation_cutoff: f64,
        rating_retention: f64,
        max_drop: f64,
        participation_limit: f64,
    ) -> f64 {
        if player.rating.rating < participation_cutoff {
            return player.rating.rating;
        }
        let participation =
            20.0 * (0.5f64).powf((date as f64 - player.rating.rating_age) / rating_retention);
        if player.rating.rating < participation_cutoff + max_drop {
            return player
                .rating
                .rating
                .min(participation_cutoff + (max_drop * participation) / participation_limit);
        } else {
            return (player.rating.rating
                - (max_drop * (1.0 - participation / participation_limit)))
                .min(player.rating.rating);
        }
    }

    fn adjust_player(
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

    async fn adjust_ratings(&self) -> ServiceResult<()> {
        const INITIAL_RATING: f64 = 1000.0;
        const BONUS_RATING: f64 = 750.0;
        const BONUS_FACTOR: f64 = 60.0;
        const PARTICIPATION_LIMIT: f64 = 10.0;
        const PARTICIPATION_CUTOFF: f64 = 1500.0;
        const MAX_DROP: f64 = 200.0;
        const RATING_RETENTION: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 240.0;

        let games: Vec<(GameId, GameRecord)> = self.game_repository.get_games().await?;
        for (game_id, game_record) in games {
            if !self.is_game_eligible_for_rating(&game_record) {
                continue;
            }
            let white: (Option<PlayerId>, Player) =
                self.player_service.fetch_player(&game_record.white).await?;
            let (Some(white_id), white_player) = white else {
                continue;
            };
            let black: (Option<PlayerId>, Player) =
                self.player_service.fetch_player(&game_record.black).await?;
            let (Some(black_id), black_player) = black else {
                continue;
            };

            let white_rating = white_player.rating.rating;
            let black_rating = black_player.rating.rating;
            let white_rating_adjusted = Self::adjusted_rating(
                white_player.clone(),
                game_record.date.timestamp(),
                PARTICIPATION_CUTOFF,
                RATING_RETENTION,
                MAX_DROP,
                PARTICIPATION_LIMIT,
            );
        }
        Ok(())
    }
}
