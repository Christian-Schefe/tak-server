use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use tak_core::{TakActionRecord, TakGameResult, TakGameSettings, TakPlayer, TakTimeSettings};

use crate::{
    UserId,
    game_record::{GameMetadata, GameRatingInfo},
};

#[derive(Clone, Debug)]
pub struct PlayerRating {
    pub user_id: UserId,
    pub rating: f64,
    pub boost: f64,
    pub max_rating: f64,
    pub rated_games_played: u32,
    pub rating_age: Option<DateTime<Utc>>,
    pub fatigue: HashMap<UserId, f64>,
}

impl PlayerRating {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            rating: 1000.0,
            boost: 750.0,
            max_rating: 1000.0,
            rated_games_played: 0,
            rating_age: None,
            fatigue: HashMap::new(),
        }
    }
}

const INITIAL_RATING: f64 = 1000.0;
const BONUS_RATING: f64 = 750.0;
const BONUS_FACTOR: f64 = 60.0;
const RATING_RETENTION: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 240.0;

fn is_game_eligible_for_rating(settings: &TakGameSettings, ply_count: usize) -> bool {
    if ply_count <= 6 {
        return false;
    }
    let TakTimeSettings::Realtime(time_control) = &settings.time_settings else {
        return false;
    };
    if settings.base.board_size < 5 {
        return false;
    }
    const TIME_LIMITS: [u32; 4] = [180, 240, 300, 360];
    const PIECE_LIMITS: [(u32, u32); 4] = [(20, 32), (25, 40), (30, 48), (40, 64)];
    const CAPSTONE_LIMITS: [(u32, u32); 4] = [(1, 1), (1, 2), (1, 2), (1, 3)];

    let size_index = ((settings.base.board_size - 5) as usize).min(3);

    let contingent_secs = time_control.contingent.as_secs();
    let time_score = contingent_secs * 3 + time_control.increment.as_secs();
    if time_score < TIME_LIMITS[size_index] as u64 || contingent_secs < 60 {
        return false;
    }
    let reserve = &settings.base.reserve;
    if reserve.pieces < PIECE_LIMITS[size_index].0 || reserve.pieces > PIECE_LIMITS[size_index].1 {
        return false;
    }
    if reserve.capstones < CAPSTONE_LIMITS[size_index].0
        || reserve.capstones > CAPSTONE_LIMITS[size_index].1
    {
        return false;
    }

    true
}

fn update_rating(
    player: &mut PlayerRating,
    amount: f64,
    fairness: f64,
    fatigue_factor: f64,
    date: DateTime<Utc>,
    bonus_factor: f64,
    bonus_rating: f64,
    rating_retention: f64,
    initial_rating: f64,
) {
    let bonus = f64::min(
        f64::max(
            0.0,
            (fatigue_factor * amount * f64::max(player.boost, 1.0) * bonus_factor) / bonus_rating,
        ),
        player.boost,
    );
    player.boost -= bonus;
    let k = 10.0
        + 15.0 * (0.5f64).powf(player.rated_games_played as f64 / 200.0)
        + 15.0 * (0.5f64).powf((player.max_rating - initial_rating) / 300.0);
    player.rating += fatigue_factor * amount * k + bonus;

    let time_decay = player
        .rating_age
        .map(|dt| {
            (date.signed_duration_since(dt).num_milliseconds().max(0) as f64) / rating_retention
        })
        .unwrap_or(1.0);
    let participation = f64::min(
        20.0,
        20.0 * (0.5f64).powf(time_decay) + fairness * fatigue_factor,
    );
    let extra_millis = f64::log2(participation / 20.0) * rating_retention;
    player.rating_age = Some(
        date.checked_add_signed(TimeDelta::milliseconds(extra_millis as i64))
            .unwrap_or(date),
    );
    player.rated_games_played += 1;
    player.max_rating = f64::max(player.max_rating, player.rating);
}

fn update_fatigue(player: &mut PlayerRating, opponent_id: UserId, game_factor: f64) {
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
    opponent_id: UserId,
    amount: f64,
    fairness: f64,
    fatigue_factor: f64,
    date: DateTime<Utc>,
) {
    update_rating(
        player,
        amount,
        fairness,
        fatigue_factor,
        date,
        BONUS_FACTOR,
        BONUS_RATING,
        RATING_RETENTION,
        INITIAL_RATING,
    );
    update_fatigue(player, opponent_id, fairness * fatigue_factor);
}

pub fn calculate_ratings(
    metadata: &GameMetadata,
    result: TakGameResult,
    history: &[TakActionRecord],
    white_rating: &mut PlayerRating,
    black_rating: &mut PlayerRating,
) -> Option<GameRatingInfo> {
    if !metadata.is_rated {
        return None;
    }
    let white_id = metadata.white.user_id;
    let black_id = metadata.black.user_id;

    if !is_game_eligible_for_rating(&metadata.settings, history.len()) {
        return None;
    }

    let result = match result {
        TakGameResult::Win { winner, .. } => match winner {
            TakPlayer::White => 1.0,
            TakPlayer::Black => 0.0,
        },
        TakGameResult::Draw => 0.5,
        TakGameResult::Aborted => return None,
    };

    let old_white_rating = white_rating.rating;
    let old_black_rating = black_rating.rating;

    let sw = 10f64.powf(white_rating.rating / 400.0);
    let sb = 10f64.powf(black_rating.rating / 400.0);
    let expected = sw / (sw + sb);
    let fairness = expected * (1.0 - expected);
    let fatigue_factor = (1.0 - white_rating.fatigue.get(&black_id).unwrap_or(&0.0) * 0.4)
        * (1.0 - black_rating.fatigue.get(&white_id).unwrap_or(&0.0) * 0.4);
    let adjustment = result - expected;
    update_rating_and_fatigue(
        white_rating,
        black_id,
        adjustment,
        fairness,
        fatigue_factor,
        metadata.date,
    );
    update_rating_and_fatigue(
        black_rating,
        white_id,
        -adjustment,
        fairness,
        fatigue_factor,
        metadata.date,
    );

    Some(GameRatingInfo {
        rating_change_white: white_rating.rating - old_white_rating,
        rating_change_black: black_rating.rating - old_black_rating,
    })
}
