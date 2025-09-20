use std::collections::VecDeque;

use crate::tak::{TakDir, TakPlayer, TakPos, TakVariant};

#[derive(Clone, Debug)]
pub struct TakStack {
    variant: TakVariant,
    composition: Vec<TakPlayer>,
}

#[derive(Clone, Debug)]
pub struct TakBoard {
    size: u32,
    stacks: Vec<Option<TakStack>>,
}

impl TakBoard {
    pub fn new(size: u32) -> Self {
        let board_area = size * size;
        TakBoard {
            size,
            stacks: vec![None; board_area as usize],
        }
    }

    pub fn can_do_place(&self, pos: &TakPos) -> Result<(), String> {
        if !pos.is_valid(self.size) {
            return Err("Position out of bounds".to_string());
        }
        let index = (pos.y * self.size as i32 + pos.x) as usize;
        if self.stacks[index].is_some() {
            return Err("Position already occupied".to_string());
        }
        Ok(())
    }

    pub fn do_place(
        &mut self,
        pos: &TakPos,
        variant: &TakVariant,
        player: &TakPlayer,
    ) -> Result<(), String> {
        self.can_do_place(&pos)?;
        let index = (pos.y * self.size as i32 + pos.x) as usize;
        self.stacks[index] = Some(TakStack {
            variant: variant.clone(),
            composition: vec![player.clone()],
        });
        Ok(())
    }

    pub fn can_do_move(&self, pos: &TakPos, dir: &TakDir, drops: &[u32]) -> Result<(), String> {
        if !pos.is_valid(self.size) {
            return Err("Position out of bounds".to_string());
        }
        let index = (pos.y * self.size as i32 + pos.x) as usize;
        let stack = self.stacks[index]
            .as_ref()
            .ok_or_else(|| "No stack at the given position".to_string())?;
        let total_pieces: u32 = stack.composition.len() as u32;
        let drops_sum: u32 = drops.iter().sum();
        if drops_sum == 0 || drops_sum > total_pieces || drops_sum > self.size {
            return Err("Invalid number of pieces to move".to_string());
        }
        let drops_len = drops.len();
        let end_pos = pos.offset(dir, drops_len as i32);
        if !end_pos.is_valid(self.size) {
            return Err("Move goes out of bounds".to_string());
        }
        for i in 0..drops_len {
            if drops[i] == 0 {
                return Err("Drop count cannot be zero".to_string());
            }
            let cur_pos = pos.offset(dir, i as i32 + 1);
            let cur_index = (cur_pos.y * self.size as i32 + cur_pos.x) as usize;

            match &self.stacks[cur_index] {
                Some(TakStack {
                    variant: TakVariant::Standing,
                    composition: _,
                }) if stack.variant != TakVariant::Capstone
                    || i < drops_len - 1
                    || drops[i] != 1 =>
                {
                    return Err("Cannot move over standing pieces".to_string());
                }
                Some(TakStack {
                    variant: TakVariant::Capstone,
                    composition: _,
                }) => {
                    return Err("Cannot move over capstone pieces".to_string());
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn do_move(&mut self, pos: &TakPos, dir: &TakDir, drops: &[u32]) -> Result<(), String> {
        self.can_do_move(pos, dir, drops)?;
        let index = (pos.y * self.size as i32 + pos.x) as usize;
        let stack = self.stacks[index].as_mut().unwrap();
        let total_pieces: u32 = stack.composition.len() as u32;
        let drops_sum: u32 = drops.iter().sum();
        let variant = stack.variant.clone();
        let mut moving_pieces = if drops_sum == total_pieces {
            self.stacks[index].take().unwrap().composition
        } else {
            stack
                .composition
                .split_off((total_pieces - drops_sum) as usize)
        };
        moving_pieces.reverse();

        let drops_len = drops.len();
        for i in 0..drops_len {
            let current_pos = pos.offset(dir, i as i32 + 1);
            let cur_index = (current_pos.y * self.size as i32 + current_pos.x) as usize;
            let cur_stack = self.stacks[cur_index].get_or_insert(TakStack {
                variant: TakVariant::Flat,
                composition: vec![],
            });
            let to_drop = moving_pieces
                .drain(moving_pieces.len() - drops[i] as usize..)
                .rev();
            cur_stack.composition.extend(to_drop);
            if i == drops_len - 1 {
                cur_stack.variant = variant.clone();
            }
        }
        Ok(())
    }

    pub fn is_full(&self) -> bool {
        self.stacks.iter().all(|s| s.is_some())
    }

    pub fn count_flats(&self) -> (u32, u32) {
        let mut white_flats = 0;
        let mut black_flats = 0;
        for stack in &self.stacks {
            if let Some(s) = stack {
                if let Some(top_player) = s.composition.last() {
                    match s.variant {
                        TakVariant::Flat => match top_player {
                            TakPlayer::White => white_flats += 1,
                            TakPlayer::Black => black_flats += 1,
                        },
                        _ => {}
                    }
                }
            }
        }
        (white_flats, black_flats)
    }

    fn is_road_square(&self, pos: &TakPos, player: &TakPlayer) -> bool {
        if !pos.is_valid(self.size) {
            return false;
        }
        let index = (pos.y * self.size as i32 + pos.x) as usize;
        if let Some(stack) = &self.stacks[index] {
            if let Some(top_player) = stack.composition.last() {
                return stack.variant != TakVariant::Standing && top_player == player;
            }
        }
        false
    }

    pub fn check_for_road(&self, player: &TakPlayer) -> bool {
        self.find_road(true, player) || self.find_road(false, player)
    }

    fn find_road(&self, horizontal: bool, player: &TakPlayer) -> bool {
        let mut visited = vec![false; (self.size * self.size) as usize];
        let mut queue = VecDeque::new();

        for i in 0..self.size as i32 {
            let start_pos = if horizontal {
                TakPos { x: 0, y: i }
            } else {
                TakPos { x: i, y: 0 }
            };
            if self.is_road_square(&start_pos, player) {
                queue.push_back(start_pos.clone());
                let index = (start_pos.y * self.size as i32 + start_pos.x) as usize;
                visited[index] = true;
            }
        }

        while let Some(current_pos) = queue.pop_front() {
            let is_end = if horizontal {
                current_pos.x == self.size as i32 - 1
            } else {
                current_pos.y == self.size as i32 - 1
            };
            if is_end {
                return true;
            }

            for dir in &[TakDir::Up, TakDir::Down, TakDir::Left, TakDir::Right] {
                let neighbor = current_pos.offset(dir, 1);
                if self.is_road_square(&neighbor, player) {
                    let index = (neighbor.y * self.size as i32 + neighbor.x) as usize;
                    if !visited[index] {
                        queue.push_back(neighbor.clone());
                        visited[index] = true;
                    }
                }
            }
        }

        false
    }

    pub fn compute_hash_string(&self) -> String {
        let mut hash = Vec::new();
        for stack in &self.stacks {
            match stack {
                Some(s) => {
                    let variant_char = match s.variant {
                        TakVariant::Flat => 'F',
                        TakVariant::Standing => 'S',
                        TakVariant::Capstone => 'C',
                    };
                    let composition_str: String = s
                        .composition
                        .iter()
                        .map(|p| match p {
                            TakPlayer::White => 'W',
                            TakPlayer::Black => 'B',
                        })
                        .collect();
                    hash.push(format!("{}{}", variant_char, composition_str));
                }
                None => {
                    hash.push("N".to_string());
                }
            }
        }
        hash.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_and_move() {
        let mut board = TakBoard::new(5);
        let pos = TakPos { x: 0, y: 0 };
        let player = TakPlayer::White;
        let variant = TakVariant::Flat;

        assert!(board.do_place(&pos, &variant, &player).is_ok());
        assert!(board.can_do_move(&pos, &TakDir::Right, &[1]).is_ok());
        assert!(board.do_move(&pos, &TakDir::Right, &[1]).is_ok());

        let new_pos = TakPos { x: 1, y: 0 };
        assert!(board.stacks[(new_pos.y * board.size as i32 + new_pos.x) as usize].is_some());
    }

    #[test]
    fn test_block() {
        let mut board = TakBoard::new(5);
        let pos = TakPos { x: 1, y: 1 };
        let wall_pos = TakPos { x: 2, y: 1 };
        let cap_pos = TakPos { x: 1, y: 2 };
        let player = TakPlayer::White;
        let variant = TakVariant::Flat;

        assert!(board.do_place(&pos, &variant, &player).is_ok());
        assert!(
            board
                .do_place(&wall_pos, &TakVariant::Standing, &player)
                .is_ok()
        );
        assert!(
            board
                .do_place(&cap_pos, &TakVariant::Capstone, &player)
                .is_ok()
        );
        assert!(board.can_do_move(&pos, &TakDir::Right, &[1]).is_err());
        assert!(board.can_do_move(&pos, &TakDir::Up, &[1]).is_err());
    }

    #[test]
    fn test_caps_smash() {
        let mut board = TakBoard::new(5);
        let pos = TakPos { x: 0, y: 0 };
        let cap_pos = TakPos { x: 1, y: 0 };
        let player = TakPlayer::White;
        let variant = TakVariant::Capstone;

        assert!(board.do_place(&pos, &variant, &player).is_ok());
        assert!(
            board
                .do_place(&cap_pos, &TakVariant::Standing, &player)
                .is_ok()
        );
        assert_eq!(board.can_do_move(&pos, &TakDir::Right, &[1]), Ok(()));
        assert!(board.do_move(&pos, &TakDir::Right, &[1]).is_ok());
    }

    #[test]
    fn test_road_detection() {
        let mut board = TakBoard::new(5);
        let player = TakPlayer::White;
        let variant = TakVariant::Flat;

        for x in 0..5 {
            let pos = TakPos { x, y: 0 };
            assert!(board.do_place(&pos, &variant, &player).is_ok());
        }

        assert!(board.check_for_road(&player));
    }
}
