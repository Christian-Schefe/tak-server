use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use tak_core::{
    TakBaseGameSettings, TakPlayer, TakReserve,
    ptn::{action_from_ptn, action_to_ptn},
};
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
    search_queue: VecDeque<EngineSearch>,
    current_search: Option<EngineSearch>,
    is_stopping: bool,
    current_variations: HashMap<usize, Variation>,
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
pub fn search_position(settings: String, tps: String) {
    ENGINE.with(|x| {
        x.borrow_mut()
            .as_mut()
            .expect("Should be initialized")
            .search_position(settings, tps);
    });
}

#[wasm_bindgen]
pub fn is_settings_supported(settings: String) -> bool {
    if let Ok(settings) = serde_json::from_str::<GameSettingsInfoBase>(&settings) {
        Engine::are_settings_supported(&settings.to_base_settings())
    } else {
        false
    }
}

#[derive(Clone)]
pub struct EngineSearch {
    settings: GameSettingsInfoBase,
    tps: String,
    player: TakPlayer,
}

impl Engine {
    pub fn new() -> Engine {
        let sender = run(&handle_output);
        Engine {
            sender,
            search_queue: VecDeque::new(),
            current_search: None,
            current_variations: HashMap::new(),
            is_stopping: false,
        }
    }

    fn handle_output(&mut self, output: &str) {
        let Some(current_search) = &self.current_search else {
            return;
        };
        if output.starts_with("bestmove") {
            self.start_next_search();
        } else if output.starts_with("info") {
            if let Some((variation_index, variation)) =
                Self::handle_eval_info(current_search, output)
            {
                self.current_variations.insert(variation_index, variation);
                self.send_changed_variations();
            }
        }
    }

    fn handle_eval_info(search: &EngineSearch, output: &str) -> Option<(usize, Variation)> {
        let words = output.split_whitespace().collect::<Vec<_>>();
        let Some(score) = words
            .iter()
            .position(|&w| w == "score")
            .and_then(|score_index| {
                words
                    .get(score_index + 1)
                    .filter(|&&s| s == "cp")
                    .and_then(|_| {
                        words
                            .get(score_index + 2)
                            .and_then(|x| x.parse::<f64>().ok())
                    })
            })
        else {
            return None;
        };
        let multipv_index = words
            .iter()
            .position(|&w| w == "multipv")
            .and_then(|i| words.get(i + 1))
            .and_then(|x| x.parse::<usize>().ok());
        let pv = words.iter().position(|&w| w == "pv").map(|pv_index| {
            words[pv_index + 1..]
                .iter()
                .map_while(|&word| {
                    if let Some(action) = action_from_ptn(word) {
                        Some(action_to_ptn(&action))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        });

        let variation = Variation {
            moves: pv.clone().unwrap_or_default(),
            evaluation: if search.player == TakPlayer::White {
                score
            } else {
                -score
            },
        };
        Some((multipv_index.unwrap_or(0), variation))
    }

    fn send_changed_variations(&mut self) {
        if !self.current_variations.is_empty() {
            let mut variations = self.current_variations.iter().collect::<Vec<_>>();
            variations.sort_by_key(|(index, _)| *index);
            send_output(Output::Evaluation {
                evaluation: Evaluation {
                    variations: variations.into_iter().map(|(_, v)| v.clone()).collect(),
                },
            });
        }
    }

    fn search_position(&mut self, settings: String, tps: String) {
        self.stop_searching();

        let tps_words = tps.split_whitespace().collect::<Vec<_>>();

        let player = match *tps_words.get(1).unwrap_or(&"") {
            "1" => TakPlayer::White,
            "2" => TakPlayer::Black,
            player => {
                console_error(&format!("Invalid player: {}", player));
                return;
            }
        };

        let Ok(settings) = serde_json::from_str::<GameSettingsInfoBase>(&settings) else {
            console_error(&format!("Failed to parse game settings: {}", settings));
            return;
        };

        if !Self::are_settings_supported(&settings.to_base_settings()) {
            console_error(&format!("Unsupported game settings"));
            return;
        }

        self.search_queue.push_back(EngineSearch {
            settings,
            tps,
            player,
        });
        if self.current_search.is_none() {
            self.start_next_search();
        }
    }

    fn start_next_search(&mut self) {
        self.is_stopping = false;
        self.current_search = if let Some(next_search) = self.search_queue.pop_front() {
            self.send_tei(format!("teinewgame {}", next_search.settings.board_size));
            self.send_tei(format!(
                "setoption name HalfKomi value {}",
                next_search.settings.half_komi
            ));
            self.send_tei(format!("setoption name MultiPV value {}", 3));
            self.send_tei(format!("position tps {}", next_search.tps));
            self.send_tei("go infinite".to_string());
            Some(next_search)
        } else {
            None
        };
        self.current_variations.clear();
    }

    fn stop_searching(&mut self) {
        if self.current_search.is_some() && !self.is_stopping {
            self.send_tei("stop".to_string());
            self.is_stopping = true;
        }
    }

    fn send_tei(&mut self, input: String) {
        if let Err(e) = self.sender.try_send(input) {
            console_error(&format!("Failed to send message to TEI: {}", e));
        }
    }

    fn are_settings_supported(settings: &TakBaseGameSettings) -> bool {
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
    Evaluation {
        #[serde(flatten)]
        evaluation: Evaluation,
    },
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Evaluation {
    pub variations: Vec<Variation>,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Variation {
    pub moves: Vec<String>,
    pub evaluation: f64,
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
