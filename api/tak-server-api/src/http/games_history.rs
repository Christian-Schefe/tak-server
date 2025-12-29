use std::time::Duration;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use tak_core::{
    TakAction, TakActionRecord, TakGameState, TakPlayer, TakPos, TakVariant, TakWinReason,
    ptn::game_state_to_string,
};
use tak_server_app::{
    domain::{
        FinishedGameId, GameType, Pagination, SortOrder,
        game_history::{
            DateSelector, GameIdSelector, GamePlayerFilter, GameQuery, GameRecord, GameSortBy,
        },
    },
    workflow::history::query::GameQueryError,
};

use crate::{
    app::ServiceError,
    http::{AppState, PaginatedResponse},
};

pub async fn get_all(
    State(app_state): State<AppState>,
    Query(filter): Query<JsonGameRecordFilter>,
) -> Result<Json<PaginatedResponse<JsonGameRecord>>, ServiceError> {
    let id_selector = filter
        .id
        .as_ref()
        .and_then(|id_str| {
            let id_str = id_str.trim();
            if id_str.is_empty() {
                return None;
            }
            if id_str.contains("-") {
                let parts: Vec<&str> = id_str.split('-').filter(|s| !s.is_empty()).collect();
                if parts.len() == 2
                    && let (Ok(start), Ok(end)) = (parts[0].parse(), parts[1].parse())
                {
                    return Some(Ok(GameIdSelector::Range(
                        FinishedGameId(start),
                        FinishedGameId(end),
                    )));
                } else {
                    return Some(Err(ServiceError::BadRequest(
                        "Invalid ID range format".to_string(),
                    )));
                }
            } else {
                let ids: Vec<FinishedGameId> = id_str
                    .split(',')
                    .filter_map(|s| s.parse().ok().map(FinishedGameId))
                    .collect();
                Some(Ok(GameIdSelector::List(ids)))
            }
        })
        .transpose()?;
    let date_selector = filter
        .date
        .as_ref()
        .and_then(|date_str| {
            let date_str = date_str.trim();
            if date_str.is_empty() {
                return None;
            }
            if date_str.contains("-") {
                let parts: Vec<&str> = date_str.split('-').filter(|s| !s.is_empty()).collect();
                if parts.len() == 2
                    && let (Ok(start), Ok(end)) = (parts[0].parse(), parts[1].parse())
                {
                    return Some(Ok(DateSelector::Range(start, end)));
                } else {
                    return Some(Err(ServiceError::BadRequest(
                        "Invalid ID range format".to_string(),
                    )));
                }
            } else {
                let is_after = if date_str.starts_with('<') {
                    false
                } else if date_str.starts_with('>') {
                    true
                } else {
                    return Some(Err(ServiceError::BadRequest(
                        "Invalid date selector format".to_string(),
                    )));
                };
                let date_part = &date_str[1..];
                match date_part.parse() {
                    Ok(date) => {
                        if is_after {
                            Some(Ok(DateSelector::After(date)))
                        } else {
                            Some(Ok(DateSelector::Before(date)))
                        }
                    }
                    Err(_) => Some(Err(ServiceError::BadRequest(
                        "Invalid date format".to_string(),
                    ))),
                }
            }
        })
        .transpose()?;
    let game_states = filter
        .game_result
        .as_ref()
        .map(|res_str| {
            let res_str = res_str.trim().to_lowercase();
            match res_str.as_str() {
                "X-0" => Ok(TakWinReason::ALL
                    .iter()
                    .map(|reason| TakGameState::Win {
                        winner: TakPlayer::White,
                        reason: reason.clone(),
                    })
                    .collect()),
                "F-0" => Ok(vec![TakGameState::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                }]),
                "R-0" => Ok(vec![TakGameState::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Road,
                }]),
                "1-0" => Ok(vec![TakGameState::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Default,
                }]),
                "0-X" => Ok(TakWinReason::ALL
                    .iter()
                    .map(|reason| TakGameState::Win {
                        winner: TakPlayer::Black,
                        reason: reason.clone(),
                    })
                    .collect()),
                "0-F" => Ok(vec![TakGameState::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                }]),
                "0-R" => Ok(vec![TakGameState::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Road,
                }]),
                "0-1" => Ok(vec![TakGameState::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Default,
                }]),
                "1/2-1/2" => Ok(vec![TakGameState::Draw]),
                _ => Err(ServiceError::BadRequest(
                    "Invalid game result filter".to_string(),
                )),
            }
        })
        .transpose()?;
    let game_type = filter
        .game_type
        .as_ref()
        .and_then(|type_str| {
            let type_str = type_str.trim().to_lowercase();
            match type_str.as_str() {
                "" => None,
                "normal" => Some(Ok(GameType::Rated)),
                "unrated" => Some(Ok(GameType::Unrated)),
                "tournament" => Some(Ok(GameType::Tournament)),
                _ => Some(Err(ServiceError::BadRequest(
                    "Invalid game type filter".to_string(),
                ))),
            }
        })
        .transpose()?;
    let page = filter.page.unwrap_or(0);
    let limit = filter.limit.filter(|&l| l > 0).unwrap_or(50);
    let skip = filter.skip.unwrap_or(0);
    let offset = if page > 1 { (page - 1) * limit } else { skip };
    let pagination = Pagination {
        offset: Some(offset),
        limit: Some(limit),
    };
    let sort = filter
        .sort
        .as_ref()
        .and_then(|sort_str| {
            let sort_str = sort_str.trim().to_lowercase();
            match sort_str.as_str() {
                "" => None,
                "date" => Some(Ok(GameSortBy::Date)),
                "id" => Some(Ok(GameSortBy::GameId)),
                _ => Some(Err(ServiceError::BadRequest(
                    "Invalid sort order".to_string(),
                ))),
            }
        })
        .transpose()?
        .unwrap_or(GameSortBy::GameId);

    let order = filter
        .order
        .as_ref()
        .and_then(|order_str| match order_str.trim().to_lowercase().as_str() {
            "asc" => Some(Ok(SortOrder::Ascending)),
            "desc" => Some(Ok(SortOrder::Descending)),
            "" => None,
            _ => Some(Err(ServiceError::BadRequest(
                "Invalid sort order".to_string(),
            ))),
        })
        .transpose()?
        .unwrap_or(SortOrder::Descending);

    let clock_contingent = filter.timertime.map(|x| Duration::from_secs(x as u64));
    let clock_increment = filter.timerinc.map(|x| Duration::from_secs(x as u64));
    let clock_extra_time = filter
        .extra_time_amount
        .map(|x| Duration::from_secs(x as u64));

    let mut game_filter = GameQuery {
        id_selector,
        date_selector,
        player_white: filter.player_white.map(|s| GamePlayerFilter::Contains(s)),
        player_black: filter.player_black.map(|s| GamePlayerFilter::Contains(s)),
        game_states,
        half_komi: filter.komi,
        board_size: filter.size,
        game_type,
        clock_contingent,
        clock_increment,
        clock_extra_trigger: filter.extra_time_trigger,
        clock_extra_time,
        pagination,
        sort: Some((order, sort)),
    };

    if filter.mirror {
        std::mem::swap(&mut game_filter.player_white, &mut game_filter.player_black);
        game_filter.game_states.as_mut().map(|states| {
            states.iter_mut().for_each(|state| match state {
                TakGameState::Win { winner, .. } => {
                    *winner = match winner {
                        TakPlayer::White => TakPlayer::Black,
                        TakPlayer::Black => TakPlayer::White,
                    }
                }
                _ => {}
            });
        });
    }

    let res = match app_state
        .app
        .game_history_query_use_case
        .query_games(game_filter)
        .await
    {
        Ok(result) => result,
        Err(GameQueryError::RepositoryError) => {
            return Err(ServiceError::Internal(format!("Error querying games")));
        }
    };

    let total = res.total_count;
    Ok(Json(PaginatedResponse {
        items: res
            .items
            .into_iter()
            .map(|(id, record)| JsonGameRecord::from_game_record(id, &record))
            .collect(),
        total,
        page,
        per_page: limit,
        total_pages: if limit > 0 {
            (total + limit - 1) / limit
        } else {
            1
        },
    }))
}

#[derive(serde::Deserialize)]
pub struct JsonGameRecordFilter {
    limit: Option<usize>,
    page: Option<usize>,
    skip: Option<usize>,
    order: Option<String>,
    sort: Option<String>,
    id: Option<String>,
    player_white: Option<String>,
    player_black: Option<String>,
    game_result: Option<String>,
    size: Option<usize>,
    #[serde(rename = "type")]
    game_type: Option<String>,
    #[serde(default)]
    mirror: bool,
    date: Option<String>,
    komi: Option<usize>,
    timertime: Option<usize>,
    timerinc: Option<usize>,
    extra_time_amount: Option<usize>,
    extra_time_trigger: Option<usize>,
}

pub async fn get_by_id(
    Path(game_id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<JsonGameRecord>, ServiceError> {
    let game_id = FinishedGameId(
        game_id
            .parse()
            .map_err(|e| ServiceError::BadRequest(format!("Invalid game ID: {}", e)))?,
    );
    let res: Option<GameRecord> = match app_state
        .app
        .game_history_query_use_case
        .get_game(game_id)
        .await
    {
        Ok(record) => record,
        Err(GameQueryError::RepositoryError) => {
            return Err(ServiceError::Internal(format!("Error retrieving game")));
        }
    };
    if let Some(record) = res {
        let json_record = JsonGameRecord::from_game_record(game_id, &record);
        let json = serde_json::to_string(&json_record).map_err(|e| {
            ServiceError::Internal(format!("Failed to serialize game record: {}", e))
        })?;
        println!("{}", json);
        Ok(Json(json_record))
    } else {
        Err(ServiceError::NotFound(format!(
            "Game with ID {} not found",
            game_id.0
        )))
    }
}

pub async fn get_ptn_by_id(
    Path(game_id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<String, ServiceError> {
    let game_id = FinishedGameId(
        game_id
            .parse()
            .map_err(|e| ServiceError::BadRequest(format!("Invalid game ID: {}", e)))?,
    );
    let res: Option<GameRecord> = match app_state
        .app
        .game_history_query_use_case
        .get_game(game_id)
        .await
    {
        Ok(record) => record,
        Err(GameQueryError::RepositoryError) => {
            return Err(ServiceError::Internal(format!("Error retrieving game")));
        }
    };
    if let Some(record) = res {
        let ptn = tak_core::ptn::game_to_ptn(
            &record.settings,
            record.result,
            record.moves.into_iter().map(|mv| mv.action).collect(),
            (
                record
                    .white
                    .username
                    .as_deref()
                    .unwrap_or("Anonymous")
                    .to_string(),
                record.white.rating,
            ),
            (
                record
                    .black
                    .username
                    .as_deref()
                    .unwrap_or("Anonymous")
                    .to_string(),
                record.black.rating,
            ),
            record.date,
        )
        .to_string();
        Ok(ptn)
    } else {
        Err(ServiceError::NotFound(format!(
            "Game with ID {} not found",
            game_id.0
        )))
    }
}

#[derive(serde::Serialize)]
pub struct JsonGameRecord {
    id: i64,
    date: i64,
    size: u32,
    player_white: String,
    player_black: String,
    notation: String,
    result: String,
    timer_time: u32,
    timer_inc: u32,
    rating_white: f64,
    rating_black: f64,
    unrated: bool,
    tournament: bool,
    komi: u32,
    pieces: i32,
    capstones: i32,
    rating_change_white: f64,
    rating_change_black: f64,
    extra_time_amount: u64,
    extra_time_trigger: u32,
}

fn action_record_to_database_string(record: &TakActionRecord) -> String {
    fn square_to_string(pos: &TakPos) -> String {
        format!(
            "{}{}",
            (b'A' + pos.x as u8) as char,
            (b'1' + pos.y as u8) as char,
        )
    }
    match &record.action {
        TakAction::Place { pos, variant } => format!(
            "P {} {}",
            square_to_string(pos),
            match variant {
                TakVariant::Flat => "",
                TakVariant::Standing => "S",
                TakVariant::Capstone => "C",
            },
        ),
        TakAction::Move { pos, dir, drops } => {
            let to_pos = pos.offset(dir, drops.len() as i32);
            let drops_str = drops
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("");
            format!(
                "M {} {} {}",
                square_to_string(pos),
                square_to_string(&to_pos),
                drops_str
            )
        }
    }
}

impl JsonGameRecord {
    fn from_game_record(id: FinishedGameId, record: &GameRecord) -> Self {
        Self {
            id: id.0,
            date: record.date.timestamp(),
            size: record.settings.board_size,
            player_white: record
                .white
                .username
                .as_deref()
                .unwrap_or("Anonymous")
                .to_string(),
            player_black: record
                .black
                .username
                .as_deref()
                .unwrap_or("Anonymous")
                .to_string(),
            notation: record
                .moves
                .iter()
                .map(|mv| action_record_to_database_string(mv))
                .collect::<Vec<_>>()
                .join(","),
            result: game_state_to_string(&record.result),
            timer_time: record.settings.time_control.contingent.as_secs() as u32,
            timer_inc: record.settings.time_control.increment.as_secs() as u32,
            rating_white: record.white.rating.unwrap_or(0.0),
            rating_black: record.black.rating.unwrap_or(0.0),
            unrated: matches!(record.game_type, GameType::Unrated),
            tournament: matches!(record.game_type, GameType::Tournament),
            komi: record.settings.half_komi,
            pieces: record.settings.reserve.pieces as i32,
            capstones: record.settings.reserve.capstones as i32,
            rating_change_white: record
                .rating_info
                .as_ref()
                .map_or(0.0, |x| x.rating_change_white),
            rating_change_black: record
                .rating_info
                .as_ref()
                .map_or(0.0, |x| x.rating_change_black),
            extra_time_amount: record
                .settings
                .time_control
                .extra
                .map_or(0, |(_, amount)| amount.as_secs()),
            extra_time_trigger: record
                .settings
                .time_control
                .extra
                .map_or(0, |(trigger, _)| trigger),
        }
    }
}
