use std::{collections::HashMap, sync::OnceLock};

use tak_core::{
    TakBaseGameSettings, TakPlayer, TakReserve,
    ptn::{action_from_ptn, action_to_ptn},
};
use tak_server_api_contract::game::GameSettingsInfoBase;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::DedicatedWorkerGlobalScope;

use crate::tei::run;

mod tei;

thread_local! {
static ENGINE: OnceLock<async_channel::Sender<Input>> = OnceLock::new();
}

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
    let (input_sender, input_receiver) = async_channel::unbounded();
    if !ENGINE.with(|x| {
        let mut did_init = false;
        x.get_or_init(|| {
            did_init = true;
            input_sender
        });
        did_init
    }) {
        return;
    }
    spawn_local(run_engine(input_receiver));
}

pub fn console_error(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}

pub fn console_log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

pub struct Engine {
    sender: async_channel::Sender<String>,
    player: TakPlayer,
    current_variations: HashMap<usize, Variation>,
}

async fn run_engine(recv: async_channel::Receiver<Input>) {
    let mut engine = Option::<(String, Engine)>::None;
    while let Ok(input) = recv.recv().await {
        match input {
            Input::SearchPosition { key, settings, tps } => {
                if let Some((_, engine)) = engine.take() {
                    engine.stop_searching();
                }
                if let Some(new_engine) = Engine::new(key.clone(), settings, tps) {
                    engine = Some((key.clone(), new_engine));
                }
            }
            Input::StopSearching => {
                if let Some((_, engine)) = engine.take() {
                    engine.stop_searching();
                }
            }
            Input::HandleOutput { key, output } => {
                if let Some((engine_key, engine)) = engine.as_mut()
                    && engine_key == &key
                {
                    engine.handle_output(&output);
                }
            }
        }
    }
}

#[wasm_bindgen]
pub fn search_position(key: String, settings: String, tps: String) {
    let settings = match serde_json::from_str::<GameSettingsInfoBase>(&settings) {
        Ok(settings) => settings,
        Err(e) => {
            console_error(&format!("Failed to parse game settings: {}", e));
            return;
        }
    };
    ENGINE.with(|x| {
        let Some(sender) = x.get() else {
            console_error("Engine not initialized");
            return;
        };
        if let Err(e) = sender.try_send(Input::SearchPosition { key, settings, tps }) {
            console_error(&format!("Failed to send search position message: {}", e));
        }
    });
}

#[wasm_bindgen]
pub fn stop_searching() {
    ENGINE.with(|x| {
        let Some(sender) = x.get() else {
            console_error("Engine not initialized");
            return;
        };
        if let Err(e) = sender.try_send(Input::StopSearching) {
            console_error(&format!("Failed to send stop searching message: {}", e));
        }
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

fn handle_output(key: String, output: &str) {
    ENGINE.with(|x| {
        let Some(sender) = x.get() else {
            console_error("Engine not initialized");
            return;
        };
        if let Err(e) = sender.try_send(Input::HandleOutput {
            key: key.clone(),
            output: output.to_string(),
        }) {
            console_error(&format!("Failed to send handle output message: {}", e));
        }
    });
}

impl Engine {
    pub fn new(key: String, settings: GameSettingsInfoBase, tps: String) -> Option<Engine> {
        let tps_words = tps.split_whitespace().collect::<Vec<_>>();

        let player = match *tps_words.get(1).unwrap_or(&"") {
            "1" => TakPlayer::White,
            "2" => TakPlayer::Black,
            player => {
                console_error(&format!("Invalid player: {}", player));
                return None;
            }
        };

        if !Self::are_settings_supported(&settings.to_base_settings()) {
            console_error(&format!("Unsupported game settings"));
            return None;
        }

        let key_clone = key.clone();
        let sender = run(move |output| handle_output(key_clone.clone(), output));

        let mut engine = Engine {
            sender,
            player,
            current_variations: HashMap::new(),
        };
        engine.send_tei("tei".to_string());
        engine.send_tei(format!("teinewgame {}", settings.board_size));
        engine.send_tei(format!(
            "setoption name HalfKomi value {}",
            settings.half_komi
        ));
        engine.send_tei(format!("setoption name MultiPV value {}", 3));
        engine.send_tei(format!("position tps {}", tps));
        engine.send_tei("go infinite".to_string());
        Some(engine)
    }

    fn handle_output(&mut self, output: &str) {
        if output.starts_with("info") {
            if let Some((variation_index, variation)) = self.handle_eval_info(output) {
                self.current_variations.insert(variation_index, variation);
                self.send_changed_variations();
            }
        }
    }

    fn handle_eval_info(&self, output: &str) -> Option<(usize, Variation)> {
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
            evaluation: if self.player == TakPlayer::White {
                score
            } else {
                -score
            },
        };
        Some((multipv_index.unwrap_or(0), variation))
    }

    fn send_changed_variations(&mut self) {
        if !self.current_variations.is_empty() {
            let mut variations = self
                .current_variations
                .values()
                .cloned()
                .collect::<Vec<_>>();
            match self.player {
                TakPlayer::White => {
                    variations.sort_by(|a, b| b.evaluation.total_cmp(&a.evaluation))
                }
                TakPlayer::Black => {
                    variations.sort_by(|a, b| a.evaluation.total_cmp(&b.evaluation))
                }
            }
            send_output(Output::Evaluation {
                evaluation: Evaluation { variations },
            });
        }
    }

    fn stop_searching(mut self) {
        self.send_tei("stop".to_string());
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Input {
    SearchPosition {
        key: String,
        settings: GameSettingsInfoBase,
        tps: String,
    },
    StopSearching,
    HandleOutput {
        key: String,
        output: String,
    },
}

#[derive(serde::Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Output {
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
