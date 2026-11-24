use crate::{PortsCollection, ports::notification::NotificationMessage};
use tak_domain::{GameId, UserId};

pub fn notify_players_and_observers_of_game<Ports: PortsCollection + Send + Sync + 'static>(
    ports: &Ports,
    game_id: GameId,
    players: &[UserId],
    message: &NotificationMessage,
) {
    let observers = ports.spectator_service().get_spectators_for_game(game_id);
    let users = [players, &observers].concat();
    ports.notification_port().notify_users(&users, message);
}
