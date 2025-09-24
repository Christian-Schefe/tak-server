use crate::tak::{TakAction, TakDir, TakPos, TakVariant};

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

pub fn ptn_to_action(ptn: &str) -> Option<TakAction> {
    let chars = ptn.trim().chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return None;
    }
    let move_idx = chars
        .iter()
        .position(|&c| c == '+' || c == '-' || c == '<' || c == '>');
    if let Some(move_idx) = move_idx {
        let (take_str, rest) = chars.split_at(move_idx);
        let dir = match rest[0] {
            '+' => TakDir::Up,
            '-' => TakDir::Down,
            '<' => TakDir::Left,
            '>' => TakDir::Right,
            _ => return None,
        };
        if rest.len() < 3 {
            return None;
        }
        let pos = parse_pos(&rest[1..3])?;
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
