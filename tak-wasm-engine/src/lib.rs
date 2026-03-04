use std::cell::RefCell;

use tak_core::{TakBaseGameSettings, TakReserve};
use tak_server_api_contract::game::GameSettingsInfoBase;
use wasm_bindgen::prelude::*;
use web_sys::DedicatedWorkerGlobalScope;

use crate::tei::run;

mod tei;

thread_local! {
static ENGINE: RefCell<Option<Engine>> = RefCell::new(None);
}

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
    send_output(Output::Loaded);
    let engine = Engine::new();
    ENGINE.with(|x| {
        *x.borrow_mut() = Some(engine);
    });
    ENGINE.with(|x| {
        x.borrow_mut()
            .as_mut()
            .expect("Should be initialized")
            .send_tei("tei".to_string());
    });
}

pub fn console_error(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}

pub struct Engine {
    sender: async_channel::Sender<String>,
    is_searching: bool,
    waiting_for_stop: bool,
}

fn handle_output(output: &str) {
    ENGINE.with(|x| {
        x.borrow_mut()
            .as_mut()
            .expect("Should be initialized")
            .handle_output(output);
    });
}

#[wasm_bindgen]
pub fn search_position(settings: String, position: String) {
    ENGINE.with(|x| {
        x.borrow_mut()
            .as_mut()
            .expect("Should be initialized")
            .search_position(settings, position);
    });
}

impl Engine {
    pub fn new() -> Engine {
        let sender = run(&handle_output);
        Engine {
            sender,
            is_searching: false,
            waiting_for_stop: false,
        }
    }

    fn handle_output(&mut self, output: &str) {
        if output.starts_with("bestmove") {
            self.waiting_for_stop = false;
        }
        if output.starts_with("info") {
            if self.waiting_for_stop {
                return;
            }
            let words = output.split_whitespace().collect::<Vec<_>>();
            if let Some(score_index) = words.iter().position(|&w| w == "score") {
                if let Some("cp") = words.get(score_index + 1).map(|s| s.as_ref()) {
                    if let Some(score_str) = words.get(score_index + 2) {
                        if let Ok(score) = score_str.parse::<f64>() {
                            send_output(Output::Evaluation { score });
                        } else {
                            console_error(&format!("Failed to parse score: {}", score_str));
                        }
                    } else {
                        console_error("Score value missing in TEI output");
                    }
                }
            }
        }
    }

    fn search_position(&mut self, settings: String, tps: String) {
        self.stop_searching();

        let Ok(settings) = serde_json::from_str::<GameSettingsInfoBase>(&settings) else {
            console_error(&format!("Failed to parse game settings: {}", settings));
            return;
        };

        if !self.are_settings_supported(&settings.to_base_settings()) {
            console_error(&format!("Unsupported game settings"));
            return;
        }

        self.send_tei(format!("teinewgame {}", settings.board_size));
        self.send_tei(format!(
            "setoption name HalfKomi value {}",
            settings.half_komi
        ));

        self.send_tei(format!("position tps {}", tps));
        self.send_tei("go infinite".to_string());
        self.is_searching = true;
    }

    fn stop_searching(&mut self) {
        if self.is_searching {
            self.waiting_for_stop = true;
            self.send_tei("stop".to_string());
            self.is_searching = false;
        }
    }

    fn send_tei(&mut self, input: String) {
        if let Err(e) = self.sender.try_send(input) {
            console_error(&format!("Failed to send message to TEI: {}", e));
        }
    }

    fn are_settings_supported(&self, settings: &TakBaseGameSettings) -> bool {
        if !settings.is_valid() {
            return false;
        }
        (settings.half_komi == 0 || settings.half_komi == 4)
            && (settings.board_size >= 4 && settings.board_size <= 6)
            && TakReserve::from_size(settings.board_size).is_some_and(|x| x == settings.reserve)
    }
}

#[derive(serde::Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Output {
    Loaded,
    Evaluation { score: f64 },
}

fn worker_scope() -> DedicatedWorkerGlobalScope {
    js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>()
}

fn send_output(output: Output) {
    let scope = worker_scope();
    scope
        .post_message(&JsValue::from_str(&serde_json::to_string(&output).unwrap()))
        .unwrap();
}
