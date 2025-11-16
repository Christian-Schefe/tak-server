use std::{collections::HashMap, sync::Arc};

use tak_core::{TakGameState, TakPlayer};

use crate::{
    ServiceResult,
    game::{ArcGameRepository, GameId, GameRatingUpdate, GameRecord, GameType},
    game_history::{GameFilter, GameFilterResult},
    player::{ArcPlayerRepository, ArcPlayerService, Player, PlayerFilter, PlayerId},
};

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

#[derive(Clone, Debug)]
pub struct GameRatingInfo {
    pub rating_white: f64,
    pub rating_black: f64,
    pub rating_change: Option<(f64, f64)>,
}

pub type ArcLastUsedGameIdRepository = Arc<Box<dyn LastUsedGameIdRepository + Send + Sync>>;

#[async_trait::async_trait]
pub trait LastUsedGameIdRepository {
    async fn get_last_used_game_id(&self) -> ServiceResult<Option<GameId>>;
    async fn set_last_used_game_id(&self, game_id: GameId) -> ServiceResult<()>;
}

pub type ArcRatingService = Arc<Box<dyn RatingService + Send + Sync>>;

#[async_trait::async_trait]
pub trait RatingService {
    async fn adjust_ratings(&self) -> ServiceResult<()>;
}

pub struct RatingServiceImpl {
    player_service: ArcPlayerService,
    player_repository: ArcPlayerRepository,
    game_repository: ArcGameRepository,
    last_used_game_id_repository: ArcLastUsedGameIdRepository,
}

impl RatingServiceImpl {
    pub fn new(
        player_repository: ArcPlayerRepository,
        game_repository: ArcGameRepository,
        player_service: ArcPlayerService,
        last_used_game_id_repository: ArcLastUsedGameIdRepository,
    ) -> Self {
        Self {
            player_repository,
            game_repository,
            player_service,
            last_used_game_id_repository,
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
        let reserve = &game_record.settings.reserve;
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

        match game_record.result {
            TakGameState::Ongoing => return false,
            _ => {}
        }

        if game_record.moves.len() <= 6 {
            return false;
        }

        true
    }

    fn adjusted_rating(
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
        let participation =
            20.0 * (0.5f64).powf((date as f64 - rating.rating_age) / rating_retention);
        if rating.rating < participation_cutoff + max_drop {
            return rating
                .rating
                .min(participation_cutoff + (max_drop * participation) / participation_limit);
        } else {
            return (rating.rating - (max_drop * (1.0 - participation / participation_limit)))
                .min(rating.rating);
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
}

#[async_trait::async_trait]
impl RatingService for RatingServiceImpl {
    async fn adjust_ratings(&self) -> ServiceResult<()> {
        const INITIAL_RATING: f64 = 1000.0;
        const BONUS_RATING: f64 = 750.0;
        const BONUS_FACTOR: f64 = 60.0;
        const PARTICIPATION_LIMIT: f64 = 10.0;
        const PARTICIPATION_CUTOFF: f64 = 1500.0;
        const MAX_DROP: f64 = 200.0;
        const RATING_RETENTION: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 240.0;

        let result: GameFilterResult = self
            .game_repository
            .get_games(GameFilter::default())
            .await?;
        let games = result.games;

        let mut is_updating = true;
        let now = chrono::Utc::now();

        let mut last_used_game_id = self
            .last_used_game_id_repository
            .get_last_used_game_id()
            .await?;

        let all_players: Vec<(PlayerId, Player)> = self
            .player_service
            .get_players(PlayerFilter::default())
            .await?
            .players;

        let mut player_map: HashMap<String, (PlayerId, Player)> = all_players
            .into_iter()
            .map(|(id, player)| (player.username.clone(), (id, player)))
            .collect();

        let mut game_updates: Vec<(GameId, GameRatingInfo)> = Vec::new();

        for (game_id, game_record) in games {
            if !self.is_game_eligible_for_rating(&game_record) {
                continue;
            }
            let Some((white_id, mut white_player)): Option<(PlayerId, Player)> =
                player_map.get(&game_record.white).cloned()
            else {
                continue;
            };

            let Some((black_id, mut black_player)): Option<(PlayerId, Player)> =
                player_map.get(&game_record.black).cloned()
            else {
                continue;
            };

            let white_rating = white_player.rating.rating;
            let black_rating = black_player.rating.rating;
            let white_rating_adjusted = Self::adjusted_rating(
                &white_player.rating,
                game_record.date.timestamp(),
                PARTICIPATION_CUTOFF,
                RATING_RETENTION,
                MAX_DROP,
                PARTICIPATION_LIMIT,
            );
            let black_rating_adjusted = Self::adjusted_rating(
                &black_player.rating,
                game_record.date.timestamp(),
                PARTICIPATION_CUTOFF,
                RATING_RETENTION,
                MAX_DROP,
                PARTICIPATION_LIMIT,
            );

            let result = match &game_record.result {
                TakGameState::Win { winner, .. } => match winner {
                    TakPlayer::White => Some(1.0),
                    TakPlayer::Black => Some(0.0),
                },
                TakGameState::Draw => Some(0.5),
                _ => None,
            };

            if result.is_none()
                && now.signed_duration_since(game_record.date) < chrono::Duration::hours(6)
            {
                is_updating = false;
            }

            if let Some(result) = result
                && self.is_game_eligible_for_rating(&game_record)
                && is_updating
            {
                let sw = 10f64.powf(white_rating / 400.0);
                let sb = 10f64.powf(black_rating / 400.0);
                let expected = sw / (sw + sb);
                let fairness = expected * (1.0 - expected);
                let fatigue_factor = (1.0
                    - white_player.rating.fatigue.get(&black_id).unwrap_or(&0.0) * 0.4)
                    * (1.0 - black_player.rating.fatigue.get(&white_id).unwrap_or(&0.0) * 0.4);
                let adjustment = result - expected;
                Self::adjust_player(
                    &mut white_player.rating,
                    adjustment,
                    fairness,
                    fatigue_factor,
                    game_record.date.timestamp() as f64,
                    BONUS_FACTOR,
                    BONUS_RATING,
                    RATING_RETENTION,
                    INITIAL_RATING,
                );
                Self::adjust_player(
                    &mut black_player.rating,
                    -adjustment,
                    fairness,
                    fatigue_factor,
                    game_record.date.timestamp() as f64,
                    BONUS_FACTOR,
                    BONUS_RATING,
                    RATING_RETENTION,
                    INITIAL_RATING,
                );
                Self::update_fatigue(
                    &mut white_player.rating,
                    black_id,
                    fairness * fatigue_factor,
                );
                Self::update_fatigue(
                    &mut black_player.rating,
                    white_id,
                    fairness * fatigue_factor,
                );
                let white_rating_adjusted2 = Self::adjusted_rating(
                    &white_player.rating,
                    game_record.date.timestamp(),
                    PARTICIPATION_CUTOFF,
                    RATING_RETENTION,
                    MAX_DROP,
                    PARTICIPATION_LIMIT,
                );
                let black_rating_adjusted2 = Self::adjusted_rating(
                    &black_player.rating,
                    game_record.date.timestamp(),
                    PARTICIPATION_CUTOFF,
                    RATING_RETENTION,
                    MAX_DROP,
                    PARTICIPATION_LIMIT,
                );

                game_updates.push((
                    game_id,
                    GameRatingInfo {
                        rating_white: white_rating_adjusted,
                        rating_black: black_rating_adjusted,
                        rating_change: Some((
                            ((white_rating_adjusted2 - white_rating_adjusted) * 10.0).round(),
                            ((black_rating_adjusted2 - black_rating_adjusted) * 10.0).round(),
                        )),
                    },
                ));
                player_map.insert(white_player.username.clone(), (white_id, white_player));
                player_map.insert(black_player.username.clone(), (black_id, black_player));
            } else {
                game_updates.push((
                    game_id,
                    GameRatingInfo {
                        rating_white: white_rating_adjusted,
                        rating_black: black_rating_adjusted,
                        rating_change: None,
                    },
                ));
            }

            if is_updating {
                last_used_game_id = Some(game_id);
            }
        }

        for (_, (_, player)) in player_map.iter_mut() {
            player.rating.participation_rating = Self::adjusted_rating(
                &player.rating,
                now.timestamp(),
                PARTICIPATION_CUTOFF,
                RATING_RETENTION,
                MAX_DROP,
                PARTICIPATION_LIMIT,
            );
        }

        self.game_repository
            .update_game_ratings(
                game_updates
                    .into_iter()
                    .map(|(id, info)| (id, GameRatingUpdate { rating_info: info }))
                    .collect(),
            )
            .await?;

        self.player_repository
            .update_ratings(
                player_map
                    .into_iter()
                    .map(|(_, (id, player))| (id, player.rating))
                    .collect(),
            )
            .await?;
        if let Some(game_id) = last_used_game_id {
            self.last_used_game_id_repository
                .set_last_used_game_id(game_id)
                .await?;
        }
        Ok(())
    }
}
