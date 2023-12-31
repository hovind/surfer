use std::{collections::BTreeMap, fs, str::FromStr};

use crate::{
    util::{alpha_idx_to_uint_idx, uint_idx_to_alpha_idx},
    Message, SignalNameType, State,
};

use fzcmd::{expand_command, Command, FuzzyOutput, ParamGreed};
use itertools::Itertools;

pub fn get_parser(state: &State) -> Command<Message> {
    fn single_word(
        suggestions: Vec<String>,
        rest_command: Box<dyn Fn(&str) -> Option<Command<Message>>>,
    ) -> Option<Command<Message>> {
        Some(Command::NonTerminal(
            ParamGreed::Rest,
            suggestions,
            Box::new(move |query, _| rest_command(query)),
        ))
    }

    fn single_word_delayed_suggestions(
        suggestions: Box<dyn Fn() -> Vec<String>>,
        rest_command: Box<dyn Fn(&str) -> Option<Command<Message>>>,
    ) -> Option<Command<Message>> {
        Some(Command::NonTerminal(
            ParamGreed::Rest,
            suggestions(),
            Box::new(move |query, _| rest_command(query)),
        ))
    }

    let scopes = match &state.vcd {
        Some(v) => v
            .scopes_to_ids
            .keys()
            .map(|s| s.clone())
            .collect::<Vec<_>>(),
        None => vec![],
    };
    let signals = match &state.vcd {
        Some(v) => v
            .signals_to_ids
            .keys()
            .map(|s| s.clone())
            .collect::<Vec<_>>(),
        None => vec![],
    };
    let displayed_signals = match &state.vcd {
        Some(v) => v
            .signals
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                format!(
                    "{}_{}",
                    uint_idx_to_alpha_idx(idx, v.signals.len()),
                    v.inner.signal_from_signal_idx(s.idx).name()
                )
            })
            .collect_vec(),
        None => vec![],
    };
    let signals_in_active_scope = state
        .vcd
        .as_ref()
        .and_then(|vcd| {
            vcd.active_scope.map(|scope| {
                vcd.inner
                    .get_children_signal_idxs(scope)
                    .into_iter()
                    .map(|signal_idx| {
                        (
                            vcd.inner.signal_from_signal_idx(signal_idx).name(),
                            signal_idx,
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
            })
        })
        .unwrap_or_default();

    let color_names = state
        .config
        .theme
        .colors
        .keys()
        .map(|k| k.clone())
        .collect_vec();

    fn vcd_files() -> Vec<String> {
        if let Ok(res) = fs::read_dir(".") {
            res.map(|res| res.map(|e| e.path()).unwrap_or_default())
                .filter(|file| {
                    file.extension()
                        .map_or(false, |extension| extension.to_str().unwrap_or("") == "vcd")
                })
                .map(|file| file.into_os_string().into_string().unwrap())
                .collect::<Vec<String>>()
        } else {
            vec![]
        }
    }

    Command::NonTerminal(
        ParamGreed::Word,
        vec![
            "load_vcd",
            "load_url",
            "config_reload",
            "scroll_to_start",
            "scroll_to_end",
            "zoom_in",
            "zoom_out",
            "zoom_fit",
            "toggle_menu",
            "toggle_fullscreen",
            "module_add",
            "module_select",
            "signal_add",
            "signal_add_from_scope",
            "signal_set_color",
            "signal_set_name_type",
            "signal_force_name_type",
            "signal_focus",
            "signal_unfocus",
        ]
        .into_iter()
        .map(|s| s.into())
        .collect(),
        Box::new(move |query, _| {
            let signals_in_active_scope = signals_in_active_scope.clone();
            match query {
                "load_vcd" => single_word_delayed_suggestions(
                    Box::new(vcd_files),
                    Box::new(|word| Some(Command::Terminal(Message::LoadVcd(word.into())))),
                ),
                "load_url" => Some(Command::NonTerminal(
                    ParamGreed::Rest,
                    vec![],
                    Box::new(|query, _| {
                        Some(Command::Terminal(Message::LoadVcdFromUrl(
                            query.to_string(),
                        )))
                    }),
                )),
                "config_reload" => Some(Command::Terminal(Message::ReloadConfig)),
                "scroll_to_start" => Some(Command::Terminal(Message::ScrollToStart)),
                "scroll_to_end" => Some(Command::Terminal(Message::ScrollToEnd)),
                "zoom_in" => Some(Command::Terminal(Message::CanvasZoom {
                    mouse_ptr_timestamp: None,
                    delta: 0.5,
                })),
                "zoom_out" => Some(Command::Terminal(Message::CanvasZoom {
                    mouse_ptr_timestamp: None,
                    delta: 2.0,
                })),
                "zoom_fit" => Some(Command::Terminal(Message::ZoomToFit)),
                "toggle_menu" => Some(Command::Terminal(Message::ToggleMenu)),
                "toggle_fullscreen" => Some(Command::Terminal(Message::ToggleFullscreen)),
                // Module commands
                "module_add" => single_word(
                    scopes.clone(),
                    Box::new(|word| {
                        Some(Command::Terminal(Message::AddScope(
                            crate::ScopeDescriptor::Name(word.into()),
                        )))
                    }),
                ),
                "module_select" => single_word(
                    scopes.clone(),
                    Box::new(|word| {
                        Some(Command::Terminal(Message::SetActiveScope(
                            crate::ScopeDescriptor::Name(word.into()),
                        )))
                    }),
                ),
                // Signal commands
                "signal_add" => single_word(
                    signals.clone(),
                    Box::new(|word| {
                        Some(Command::Terminal(Message::AddSignal(
                            crate::SignalDescriptor::Name(word.into()),
                        )))
                    }),
                ),
                "signal_add_from_module" => single_word(
                    signals_in_active_scope.keys().cloned().collect(),
                    Box::new(move |name| {
                        signals_in_active_scope
                            .get(name)
                            .map(|idx| Command::Terminal(Message::AddSignal((*idx).into())))
                    }),
                ),
                "signal_set_color" => single_word(
                    color_names.clone(),
                    Box::new(|word| {
                        Some(Command::Terminal(Message::SignalColorChange(
                            None,
                            word.to_string(),
                        )))
                    }),
                ),
                "signal_set_name_type" => single_word(
                    vec![
                        "Local".to_string(),
                        "Unique".to_string(),
                        "Global".to_string(),
                    ],
                    Box::new(|word| {
                        Some(Command::Terminal(Message::ChangeSignalNameType(
                            None,
                            SignalNameType::from_str(word).unwrap_or(SignalNameType::Local),
                        )))
                    }),
                ),
                "signal_force_name_type" => single_word(
                    vec![
                        "Local".to_string(),
                        "Unique".to_string(),
                        "Global".to_string(),
                    ],
                    Box::new(|word| {
                        Some(Command::Terminal(Message::ForceSignalNameTypes(
                            SignalNameType::from_str(word).unwrap_or(SignalNameType::Local),
                        )))
                    }),
                ),
                "signal_focus" => single_word(
                    displayed_signals.clone(),
                    Box::new(|word| {
                        // split off the idx which is always followed by an underscore
                        let alpha_idx: String = word.chars().take_while(|c| *c != '_').collect();
                        alpha_idx_to_uint_idx(alpha_idx)
                            .map(|idx| Command::Terminal(Message::FocusSignal(idx)))
                    }),
                ),
                "signal_unfocus" => Some(Command::Terminal(Message::UnfocusSignal)),
                _ => None,
            }
        }),
    )
}

pub fn run_fuzzy_parser(input: &str, state: &State, msgs: &mut Vec<Message>) {
    let FuzzyOutput {
        expanded,
        suggestions,
    } = expand_command(input, get_parser(state));

    msgs.push(Message::CommandPromptUpdate {
        expanded,
        suggestions: suggestions.unwrap_or(vec![]),
    })
}
