use std::collections::HashMap;

use argh::FromArgValue;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;
use winit::{
    event::{DeviceEvent, ElementState},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::client::cl_game_actions::{Input, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ClientAction {
    ToggleConsole,
    ToggleMenu,
}

#[derive(Default)]
pub struct ClientActions {
    bindings: FxHashMap<Input, ClientAction>,
}

impl ClientActions {
    pub fn load(&mut self, source: &HashMap<String, String>) {
        self.bindings = parse_bindings(source);
    }

    fn get_from_event(&mut self, event: &DeviceEvent) -> Option<ClientAction> {
        match event {
            DeviceEvent::Key(raw_key_event) => match raw_key_event.physical_key {
                PhysicalKey::Code(key_code) if raw_key_event.state == ElementState::Pressed => {
                    let key = Input::Key(key_code);

                    return self.bindings.get(&key).copied();
                }
                _ => {}
            },
            DeviceEvent::Button { button, state } if *state == ElementState::Pressed => {
                if let Ok(button) = MouseButton::try_from(*button) {
                    let key = Input::Button(button);

                    return self.bindings.get(&key).copied();
                }
            }
            _ => {}
        }

        None
    }
}

fn parse_bindings(source: &HashMap<String, String>) -> FxHashMap<Input, ClientAction> {
    let mut bindings = FxHashMap::default();
    if let Ok(str) = toml::to_string(&source) {
        for line in str.lines() {
            if let Some((action, key)) = line.split_once("=") {
                let action = toml::de::ValueDeserializer::parse(action)
                    .and_then(|d| ClientAction::deserialize(d))
                    .ok();

                if action.is_none() {
                    continue;
                }

                let action = action.unwrap();
                let trimmed_key = key.trim();

                if let Ok(deserializer) = toml::de::ValueDeserializer::parse(trimmed_key) {
                    if let Ok(key) = KeyCode::deserialize(deserializer) {
                        bindings.insert(Input::Key(key), action);
                        continue;
                    }
                }

                if let Some(stripped) = trimmed_key
                    .strip_prefix("\"")
                    .and_then(|v| v.strip_suffix("\""))
                    .map(|v| v.to_lowercase())
                {
                    if let Ok(button) = MouseButton::from_arg_value(&stripped) {
                        bindings.insert(Input::Button(button), action);
                    }
                }
            }
        }
    } else {
        warn!("Unable to serialize bindings!");
    };
    bindings
}
