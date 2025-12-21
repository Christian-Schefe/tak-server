use crate::domain::PlayerId;

pub trait AccountRepository {
    fn set_player_silenced(&self, player_id: PlayerId, silenced: bool);
    fn is_player_silenced(&self, player_id: PlayerId) -> bool;
}
