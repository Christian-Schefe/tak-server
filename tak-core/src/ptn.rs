use std::time::Duration;

use crate::{
    TakAction, TakDir, TakGameOverState, TakGameSettings, TakPlayer, TakPos, TakReserve,
    TakTimeControl, TakVariant, TakWinReason,
};

pub enum PtnHeader {
    Size(u32),
    HalfKomi(u32),
    Player(TakPlayer, String),
    Rating(TakPlayer, f64),
    TimeControl(TakTimeControl),
    Date(chrono::DateTime<chrono::Utc>),
    Result(TakGameOverState),
    Reserve(TakReserve),
}

impl PtnHeader {
    pub fn timer_info(time: Duration, inc: Duration) -> String {
        let total_secs = time.as_secs();

        let secs = total_secs % 60;
        let mins = (total_secs / 60) % 60;
        let hrs = total_secs / 3600;

        let mut out = String::new();
        let mut force = false;

        if hrs >= 1 {
            out.push_str(&format!("{}:", hrs));
            force = true;
        }

        if mins >= 1 || force {
            out.push_str(&format!("{}:", mins));
        }

        out.push_str(&format!("{}", secs));

        if !inc.is_zero() {
            out.push_str(&format!(" +{}", inc.as_secs()));
        }

        out
    }
    pub fn to_header_string(&self) -> String {
        match self {
            PtnHeader::Size(size) => format!("[Size \"{size}\"]"),
            PtnHeader::Player(player, name) => format!(
                "[Player{} \"{name}\"]",
                match player {
                    TakPlayer::White => 1,
                    TakPlayer::Black => 2,
                }
            ),
            PtnHeader::Rating(player, rating) => format!(
                "[Rating{} \"{rating}\"]",
                match player {
                    TakPlayer::White => 1,
                    TakPlayer::Black => 2,
                }
            ),
            PtnHeader::TimeControl(tc) => format!(
                "[Clock \"{}\"]",
                PtnHeader::timer_info(tc.contingent, tc.increment)
            ),
            PtnHeader::Date(date) => format!(
                "[Date \"{}\"]\n[Time \"{}\"]",
                date.format("%Y.%m.%d"),
                date.format("%H:%M:%S")
            ),
            PtnHeader::Result(result) => {
                format!("[Result \"{}\"]", game_state_to_string(result))
            }
            PtnHeader::Reserve(reserve) => format!(
                "[Flats \"{}\"]\n[Caps \"{}\"]",
                reserve.pieces, reserve.capstones
            ),
            PtnHeader::HalfKomi(half_komi) => {
                if half_komi % 2 == 0 {
                    format!("[Komi \"{}\"]", half_komi / 2)
                } else {
                    format!("[Komi \"{}.5\"]", half_komi / 2)
                }
            }
        }
    }
}

pub struct Ptn {
    pub headers: Vec<PtnHeader>,
    pub moves: Vec<TakAction>,
}

impl Ptn {
    pub fn new(headers: Vec<PtnHeader>, moves: Vec<TakAction>) -> Self {
        Self { headers, moves }
    }
    pub fn to_string(&self) -> String {
        let mut out = String::new();
        for header in &self.headers {
            out.push_str(&header.to_header_string());
            out.push('\n');
        }
        out.push('\n');
        let mut ptn_moves: Vec<String> = Vec::new();
        for action in &self.moves {
            ptn_moves.push(action_to_ptn(action));
        }
        let pairs = ptn_moves
            .chunks(2)
            .map(|chunk| match chunk {
                [first, second] => format!(
                    "{}. {} {}",
                    (ptn_moves.iter().position(|m| m == first).unwrap() / 2) + 1,
                    first,
                    second
                ),
                [first] => format!(
                    "{}. {}",
                    (ptn_moves.iter().position(|m| m == first).unwrap() / 2) + 1,
                    first
                ),
                _ => "".to_string(),
            })
            .collect::<Vec<String>>();
        out.push_str(&pairs.join("\n"));
        out
    }
}

pub fn game_to_ptn(
    settings: &TakGameSettings,
    result: Option<TakGameOverState>,
    moves: Vec<TakAction>,
    player_white: (String, Option<f64>),
    player_black: (String, Option<f64>),
    time: chrono::DateTime<chrono::Utc>,
) -> Ptn {
    let mut headers = settings_to_ptn_headers(settings);
    headers.push(PtnHeader::Date(time));
    if let Some(result) = result {
        headers.push(PtnHeader::Result(result));
    }
    headers.push(PtnHeader::Player(TakPlayer::White, player_white.0));
    if let Some(rating) = player_white.1 {
        headers.push(PtnHeader::Rating(TakPlayer::White, rating));
    }
    headers.push(PtnHeader::Player(TakPlayer::Black, player_black.0));
    if let Some(rating) = player_black.1 {
        headers.push(PtnHeader::Rating(TakPlayer::Black, rating));
    }

    Ptn::new(headers, moves)
}

pub fn settings_to_ptn_headers(settings: &TakGameSettings) -> Vec<PtnHeader> {
    let mut headers: Vec<PtnHeader> = Vec::new();
    headers.push(PtnHeader::Size(settings.board_size));
    headers.push(PtnHeader::HalfKomi(settings.half_komi));
    headers.push(PtnHeader::Reserve(settings.reserve.clone()));
    headers.push(PtnHeader::TimeControl(settings.time_control.clone()));
    headers
}

pub fn action_to_ptn(action: &TakAction) -> String {
    match action {
        TakAction::Place { pos, variant } => {
            format!("{}{}", variant_to_string(variant), pos_to_string(pos))
        }
        TakAction::Move { pos, dir, drops } => {
            let drops_str: String = drops.iter().map(|d| d.to_string()).collect();
            let drops_str = if drops_str.len() == 1 {
                "".to_string()
            } else {
                drops_str
            };
            let take_num = drops.iter().sum::<u32>();
            let take_str = if take_num == 1 {
                "".to_string()
            } else {
                take_num.to_string()
            };
            format!(
                "{}{}{}{}",
                take_str,
                pos_to_string(pos),
                dir_to_string(dir),
                drops_str
            )
        }
    }
}

pub fn action_from_ptn(ptn: &str) -> Option<TakAction> {
    let chars = ptn.trim().chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return None;
    }
    let move_idx = chars
        .iter()
        .position(|&c| c == '+' || c == '-' || c == '<' || c == '>');
    if let Some(move_idx) = move_idx {
        if move_idx < 2 {
            return None;
        }
        let (take_str, rest) = chars.split_at(move_idx - 2);
        let dir = match rest[2] {
            '+' => TakDir::Up,
            '-' => TakDir::Down,
            '<' => TakDir::Left,
            '>' => TakDir::Right,
            _ => return None,
        };
        if rest.len() < 3 {
            return None;
        }
        let pos = parse_pos(&rest[..2])?;
        let take_num = if take_str.is_empty() {
            1
        } else {
            take_str.iter().collect::<String>().parse::<u32>().ok()?
        };
        let drops = rest[3..]
            .iter()
            .map(|c| c.to_digit(10))
            .collect::<Option<Vec<_>>>()?;
        let drops = if drops.len() == 0 {
            vec![take_num]
        } else {
            drops
        };

        Some(TakAction::Move { pos, dir, drops })
    } else {
        if chars.len() == 2 {
            let pos = parse_pos(&chars)?;
            Some(TakAction::Place {
                pos,
                variant: TakVariant::Flat,
            })
        } else if chars.len() == 3 {
            let variant = match chars[0] {
                'S' => TakVariant::Standing,
                'C' => TakVariant::Capstone,
                _ => return None,
            };
            let pos = parse_pos(&chars[1..])?;
            Some(TakAction::Place { pos, variant })
        } else {
            None
        }
    }
}

pub fn game_state_to_string(game_state: &TakGameOverState) -> String {
    match game_state {
        TakGameOverState::Win { winner, reason } => {
            let letter = match reason {
                TakWinReason::Road => "R",
                TakWinReason::Flats => "F",
                TakWinReason::Default => "1",
            };
            match winner {
                TakPlayer::White => format!("{}-0", letter),
                TakPlayer::Black => format!("0-{}", letter),
            }
        }
        TakGameOverState::Draw => "1/2-1/2".to_string(),
    }
}

pub fn game_state_from_string(s: &str) -> Option<TakGameOverState> {
    match s {
        "R-0" => Some(TakGameOverState::Win {
            winner: TakPlayer::White,
            reason: TakWinReason::Road,
        }),
        "0-R" => Some(TakGameOverState::Win {
            winner: TakPlayer::Black,
            reason: TakWinReason::Road,
        }),
        "F-0" => Some(TakGameOverState::Win {
            winner: TakPlayer::White,
            reason: TakWinReason::Flats,
        }),
        "0-F" => Some(TakGameOverState::Win {
            winner: TakPlayer::Black,
            reason: TakWinReason::Flats,
        }),
        "1-0" => Some(TakGameOverState::Win {
            winner: TakPlayer::White,
            reason: TakWinReason::Default,
        }),
        "0-1" => Some(TakGameOverState::Win {
            winner: TakPlayer::Black,
            reason: TakWinReason::Default,
        }),
        "1/2-1/2" => Some(TakGameOverState::Draw),
        _ => None,
    }
}

fn variant_to_string(variant: &TakVariant) -> &'static str {
    match variant {
        TakVariant::Flat => "",
        TakVariant::Standing => "S",
        TakVariant::Capstone => "C",
    }
}

fn pos_to_string(pos: &TakPos) -> String {
    format!("{}{}", (b'a' + pos.x as u8) as char, pos.y + 1)
}

fn dir_to_string(dir: &TakDir) -> &'static str {
    match dir {
        TakDir::Up => "+",
        TakDir::Down => "-",
        TakDir::Left => "<",
        TakDir::Right => ">",
    }
}

fn parse_pos(chars: &[char]) -> Option<TakPos> {
    if chars.len() != 2 {
        return None;
    }
    let x = chars[0];
    let y = chars[1];
    if !('a'..='z').contains(&x) || !('1'..='9').contains(&y) {
        return None;
    }
    Some(TakPos {
        x: (x as u8 - b'a') as i32,
        y: (y as u8 - b'1') as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_to_ptn() {
        let action = TakAction::Place {
            pos: TakPos::new(0, 0),
            variant: TakVariant::Flat,
        };
        assert_eq!(action_to_ptn(&action), "a1");
        let action = TakAction::Place {
            pos: TakPos::new(1, 2),
            variant: TakVariant::Standing,
        };
        assert_eq!(action_to_ptn(&action), "Sb3");
        let action = TakAction::Place {
            pos: TakPos::new(2, 4),
            variant: TakVariant::Capstone,
        };
        assert_eq!(action_to_ptn(&action), "Cc5");

        let action = TakAction::Move {
            pos: TakPos::new(3, 3),
            dir: TakDir::Up,
            drops: vec![1],
        };
        assert_eq!(action_to_ptn(&action), "d4+");
        let action = TakAction::Move {
            pos: TakPos::new(4, 4),
            dir: TakDir::Down,
            drops: vec![2],
        };
        assert_eq!(action_to_ptn(&action), "2e5-");
        let action = TakAction::Move {
            pos: TakPos::new(5, 5),
            dir: TakDir::Left,
            drops: vec![1, 2],
        };
        assert_eq!(action_to_ptn(&action), "3f6<12");
        let action = TakAction::Move {
            pos: TakPos::new(6, 6),
            dir: TakDir::Right,
            drops: vec![1, 5, 1],
        };
        assert_eq!(action_to_ptn(&action), "7g7>151");
    }

    #[test]
    fn test_ptn_to_action() {
        helper_test_ptn_to_action(
            "a1",
            Some(TakAction::Place {
                pos: TakPos::new(0, 0),
                variant: TakVariant::Flat,
            }),
        );

        helper_test_ptn_to_action(
            "Sb3",
            Some(TakAction::Place {
                pos: TakPos::new(1, 2),
                variant: TakVariant::Standing,
            }),
        );

        helper_test_ptn_to_action(
            "Cc5",
            Some(TakAction::Place {
                pos: TakPos::new(2, 4),
                variant: TakVariant::Capstone,
            }),
        );

        helper_test_ptn_to_action(
            "d4+",
            Some(TakAction::Move {
                pos: TakPos::new(3, 3),
                dir: TakDir::Up,
                drops: vec![1],
            }),
        );

        helper_test_ptn_to_action(
            "2e5-",
            Some(TakAction::Move {
                pos: TakPos::new(4, 4),
                dir: TakDir::Down,
                drops: vec![2],
            }),
        );

        helper_test_ptn_to_action(
            "3f6<12",
            Some(TakAction::Move {
                pos: TakPos::new(5, 5),
                dir: TakDir::Left,
                drops: vec![1, 2],
            }),
        );

        helper_test_ptn_to_action(
            "7g7>151",
            Some(TakAction::Move {
                pos: TakPos::new(6, 6),
                dir: TakDir::Right,
                drops: vec![1, 5, 1],
            }),
        );

        helper_test_ptn_to_action("Rc2", None);
        helper_test_ptn_to_action("a", None);
        helper_test_ptn_to_action("a0", None);
        helper_test_ptn_to_action("1c", None);
        helper_test_ptn_to_action("2c++", None);
        helper_test_ptn_to_action("Cg7+", None);
    }

    fn helper_test_ptn_to_action(ptn: &str, expected: Option<TakAction>) {
        let action = action_from_ptn(ptn);
        assert_eq!(action, expected, "ptn: {}", ptn);
    }

    #[test]
    fn test_game_state_to_string() {
        let state = TakGameOverState::Win {
            winner: TakPlayer::White,
            reason: TakWinReason::Road,
        };
        assert_eq!(game_state_to_string(&state), "R-0");

        let state = TakGameOverState::Win {
            winner: TakPlayer::Black,
            reason: TakWinReason::Flats,
        };
        assert_eq!(game_state_to_string(&state), "0-F");

        let state = TakGameOverState::Win {
            winner: TakPlayer::White,
            reason: TakWinReason::Default,
        };
        assert_eq!(game_state_to_string(&state), "1-0");

        let state = TakGameOverState::Draw;
        assert_eq!(game_state_to_string(&state), "1/2-1/2");
    }
}
