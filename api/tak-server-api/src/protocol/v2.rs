use std::{sync::Arc, time::Instant};

use tak_server_app::{
    domain::{AccountId, GameId, ListenerId, PlayerId},
    ports::{
        authentication::AuthenticationPort,
        notification::{ListenerMessage, ServerAlertMessage},
    },
};

use crate::{
    acl::LegacyAPIAntiCorruptionLayer,
    app::ServiceError,
    client::{DisconnectReason, ServerMessage, TransportServiceImpl},
    protocol::Application,
};

mod auth;
mod chat;
mod game;
mod game_list;
mod seek;
mod sudo;

pub struct ProtocolV2Handler {
    app: Arc<Application>,
    transport: Arc<TransportServiceImpl>,
    auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
    acl: Arc<LegacyAPIAntiCorruptionLayer>,
}

pub enum V2Response {
    OK,
    Message(String),
    ErrorMessage(ServiceError, String),
    ErrorNOK(ServiceError),
}

impl ProtocolV2Handler {
    pub fn new(
        app: Arc<Application>,
        transport: Arc<TransportServiceImpl>,
        auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
        acl: Arc<LegacyAPIAntiCorruptionLayer>,
    ) -> Self {
        Self {
            app,
            transport,
            auth,
            acl,
        }
    }

    pub async fn handle_client_message(&self, id: ListenerId, msg: String) {
        let parts = msg.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            log::info!("Received empty message");
            return;
        }
        let res: V2Response = match parts[0].to_ascii_lowercase().as_str() {
            "ping" => V2Response::OK,
            "protocol" => V2Response::OK, // Noop, ignore
            "client" => V2Response::OK,   // Noop, ignore
            "quit" | "exit" => {
                self.transport
                    .close_with_reason(id, DisconnectReason::ClientQuit)
                    .await;
                V2Response::OK
            }
            "login" => self.handle_login_message(id, &parts).await,
            "register" => self.handle_register_message(&parts).await,
            "sendresettoken" => self.handle_reset_token_message(&parts).await,
            "resetpassword" => self.handle_reset_password_message(&parts).await,
            _ => self.handle_logged_in_client_message(id, &parts, &msg).await,
        };
        match res {
            V2Response::Message(msg) => {
                self.send_to(id, msg);
            }
            V2Response::OK => {
                self.send_to(id, "OK");
            }
            V2Response::ErrorMessage(err, msg) => {
                log::error!("Error handling message {:?}: {}", parts, err);
                self.send_to(id, msg);
            }
            V2Response::ErrorNOK(err) => {
                log::error!("Error handling message {:?}: {}", parts, err);
                self.send_to(id, "NOK");
            }
        }
    }
    pub async fn send_server_message(&self, id: ListenerId, msg: &ServerMessage) {
        match msg {
            ServerMessage::ConnectionClosed { reason } => {
                let reason_str = match reason {
                    DisconnectReason::NewSession => {
                        "You've logged in from another window. Disconnecting"
                    }
                    DisconnectReason::Inactivity => "Disconnected due to inactivity",
                    DisconnectReason::Ban(msg) => &msg,
                    DisconnectReason::Kick => "You have been kicked from the server",
                    DisconnectReason::ServerShutdown => "Server is shutting down",
                    DisconnectReason::ClientQuit => "Quitting. Goodbye!",
                };
                let disconnect_message = format!("Message {}", reason_str);
                self.send_to(id, disconnect_message);
            }
            ServerMessage::Notification(notif) => self.send_notification_message(id, notif).await,
        }
    }

    // legacy api only notifies the opponent of draw and undo offers, not the offering player or observers
    fn is_opponent_in_game(
        &self,
        action_player_id: PlayerId,
        player_id: Option<PlayerId>,
        game_id: GameId,
    ) -> bool {
        if let Some(game) = self.app.game_get_ongoing_use_case.get_game(game_id)
            && let Some(player_id) = player_id
        {
            return (game.metadata.white_id == player_id || game.metadata.black_id == player_id)
                && player_id != action_player_id;
        }
        false
    }

    async fn send_notification_message(&self, id: ListenerId, msg: &ListenerMessage) {
        let (player_id, _) = self
            .transport
            .get_associated_player_and_account(id)
            .await
            .unzip();
        match msg {
            ListenerMessage::SeekCreated { seek } => {
                self.send_seek_list_message(id, seek, true).await;
            }
            ListenerMessage::SeekCanceled { seek } => {
                self.send_seek_list_message(id, seek, false).await;
            }

            ListenerMessage::GameStarted { game } => {
                self.send_game_list_message(id, &game.metadata, true).await;
                if let Some(player_id) = player_id
                    && (game.metadata.white_id == player_id || game.metadata.black_id == player_id)
                {
                    self.send_game_start_message(id, player_id, &game.metadata)
                        .await;
                }
            }
            ListenerMessage::GameEnded { game } => {
                self.send_game_list_message(id, &game.metadata, false).await;
            }

            ListenerMessage::GameOver {
                game_id,
                game_state,
            } => {
                self.send_game_over_message(id, *game_id, game_state);
            }
            ListenerMessage::GameAction {
                game_id,
                action,
                player_id: moving_player_id,
            } => {
                // legacy api expects only opponent to receive action messages
                if player_id.is_none_or(|x| x != *moving_player_id) {
                    self.send_game_action_message(id, *game_id, action);
                }
            }
            ListenerMessage::GameDrawOffered {
                game_id,
                offering_player_id,
            } => {
                if self.is_opponent_in_game(*offering_player_id, player_id, *game_id) {
                    self.send_draw_offer_message(id, *game_id, true);
                }
            }
            ListenerMessage::GameDrawOfferRetracted {
                game_id,
                retracting_player_id,
            } => {
                if self.is_opponent_in_game(*retracting_player_id, player_id, *game_id) {
                    self.send_draw_offer_message(id, *game_id, false);
                }
            }
            ListenerMessage::GameUndoRequested {
                game_id,
                requesting_player_id,
            } => {
                if self.is_opponent_in_game(*requesting_player_id, player_id, *game_id) {
                    self.send_undo_request_message(id, *game_id, true);
                }
            }
            ListenerMessage::GameUndoRequestRetracted {
                game_id,
                retracting_player_id,
            } => {
                if self.is_opponent_in_game(*retracting_player_id, player_id, *game_id) {
                    self.send_undo_request_message(id, *game_id, false);
                }
            }
            ListenerMessage::GameActionUndone { game_id } => {
                self.send_undo_message(id, *game_id);
            }
            ListenerMessage::GameTimeUpdate {
                game_id,
                white_time,
                black_time,
            } => {
                self.send_time_update_message(id, *game_id, *white_time, *black_time);
            }

            ListenerMessage::PlayersOnline { players } => {
                let online_message = format!("Online {}", players.len());
                let mut username_futures = Vec::new();
                for pid in players {
                    let username = self.app.get_account_workflow.get_account(*pid);
                    username_futures.push(username);
                }
                let usernames = futures::future::join_all(username_futures)
                    .await
                    .into_iter()
                    .filter_map(|x| x.ok().map(|a| a.username))
                    .collect::<Vec<_>>();
                let players_message = format!(
                    "OnlinePlayers {}",
                    serde_json::to_string(&usernames).unwrap()
                );
                self.send_to(id, online_message);
                self.send_to(id, players_message);
            }
            ListenerMessage::ChatMessage {
                from_player_id,
                message,
                source,
            } => {
                self.send_chat_message(id, *from_player_id, message, source)
                    .await
            }
            ListenerMessage::GameRematchRequested { .. } => {} //legacy api does not support rematch messages
            ListenerMessage::GameRematchRequestRetracted { .. } => {} //legacy api does not support rematch messages
            ListenerMessage::ServerAlert { message } => match message {
                ServerAlertMessage::Shutdown => {
                    self.send_to(
                        id,
                        "Server is shutting down soon. Please finish your games.",
                    );
                }
                ServerAlertMessage::Custom(msg) => {
                    self.send_to(id, msg); // This is a legacy hack, anything without a known prefix is interpreted as a server alert
                }
            },
        }
    }

    pub async fn on_authenticated(&self, id: ListenerId, account_id: &AccountId) {
        let player_id = self
            .app
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
            .ok();

        let seeks = self.app.seek_list_use_case.list_seeks();
        for seek in seeks {
            self.send_seek_list_message(id, &seek, true).await;
        }
        let games = self.app.game_list_ongoing_use_case.list_games();
        for game in games {
            self.send_game_list_message(id, &game.metadata, true).await;
            if let Some(player_id) = player_id
                && (game.metadata.white_id == player_id || game.metadata.black_id == player_id)
            {
                self.send_game_start_message(id, player_id, &game.metadata)
                    .await;
                for action in game.game.action_history() {
                    self.send_game_action_message(id, game.metadata.id, action);
                }
                let now = Instant::now();
                let (remaining_white, remaining_black) = game.game.get_time_remaining_both(now);
                self.send_time_update_message(
                    id,
                    game.metadata.id,
                    remaining_white,
                    remaining_black,
                );
            }
        }
    }

    pub fn on_connected(&self, id: ListenerId) {
        self.send_to(id, "Welcome!");
        self.send_to(id, "Login or Register");
    }

    async fn handle_logged_in_client_message(
        &self,
        id: ListenerId,
        parts: &[&str],
        msg: &str,
    ) -> V2Response {
        let Some((player_id, account_id)) =
            self.transport.get_associated_player_and_account(id).await
        else {
            return V2Response::ErrorNOK(ServiceError::Unauthorized(
                "Client is not logged in".to_string(),
            ));
        };

        match parts[0].to_ascii_lowercase().as_str() {
            "changepassword" => {
                self.handle_change_password_message(account_id, &parts)
                    .await
            }
            "seek" => self.handle_seek_message(player_id, &parts).await,
            "rematch" => self.handle_rematch_message(player_id, &parts).await,
            "list" => self.handle_seek_list_message(id).await,
            "gamelist" => self.handle_game_list_message(id).await,
            "accept" => self.handle_accept_message(player_id, &parts).await,
            "observe" => self.handle_observe_message(id, &parts, true).await,
            "unobserve" => self.handle_observe_message(id, &parts, false).await,
            "shout" => {
                self.handle_shout_message(id, account_id, player_id, &msg)
                    .await
            }
            "shoutroom" => {
                self.handle_shout_room_message(id, account_id, player_id, &msg)
                    .await
            }
            "tell" => self.handle_tell_message(account_id, player_id, &msg).await,
            "joinroom" => self.handle_room_membership_message(id, &parts, true).await,
            "leaveroom" => self.handle_room_membership_message(id, &parts, false).await,
            "sudo" => self.handle_sudo_message(id, player_id, msg, &parts).await,
            s if s.starts_with("game#") => self.handle_game_message(player_id, &parts).await,
            _ => V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                "Unknown V2 message type: {}",
                parts[0]
            ))),
        }
    }

    fn send_to<T>(&self, id: ListenerId, msg: T)
    where
        T: AsRef<str>,
    {
        let _ = self.transport.try_send_to(id, msg.as_ref());
    }
}

fn split_n_and_rest(input: &str, n: usize) -> (Vec<&str>, &str) {
    let mut parts = Vec::new();
    let mut last_pos = 0;

    // Use split_whitespace to find the first n tokens
    for (i, part) in input.split_whitespace().enumerate() {
        if i < n {
            // Find this token's position in the original string
            if let Some(pos) = input[last_pos..].find(part) {
                last_pos += pos + part.len();
            }
            parts.push(part);
        } else {
            break;
        }
    }

    // Slice from the last consumed position to keep the remainder intact
    let remainder = if last_pos < input.len() {
        &input[last_pos..] // includes original spaces
    } else {
        ""
    };

    (parts, remainder.trim_start()) // trim leading spaces in remainder
}
