use std::collections::HashMap;

use rustc_hash::FxHashMap;
use serde::Deserialize;
use tracing::warn;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::client::cl_game_actions::Input;

#[derive(Default)]
pub struct ClientActions {
    bindings: FxHashMap<Input, String>,
}

impl ClientActions {
    pub fn load(&mut self, source: &HashMap<String, String>) {
        self.bindings = parse_bindings(source);
    }

    pub fn get_from_window_event(&mut self, event: &WindowEvent) -> Option<&String> {
        match event {
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(key_code) if event.state == ElementState::Pressed => {
                    let key = Input::Key(key_code);

                    return self.bindings.get(&key);
                }
                _ => {}
            },
            WindowEvent::MouseInput { state, button, .. } if *state == ElementState::Pressed => {
                let key = Input::Button(*button);

                return self.bindings.get(&key);
            }
            _ => {}
        }

        None
    }
}

fn parse_bindings(source: &HashMap<String, String>) -> FxHashMap<Input, String> {
    let mut bindings = FxHashMap::default();
    if let Ok(str) = toml::to_string(&source) {
        for line in str.lines() {
            if let Some((key, action)) = line.split_once("=") {
                let action = action.trim();

                if action.is_empty() {
                    continue;
                }

                let action = action.trim_matches('"').to_string();
                let key = format!("\"{}\"", key.trim());
                if let Ok(deserializer) = toml::de::ValueDeserializer::parse(&key) {
                    if let Ok(btn) = MouseButton::deserialize(deserializer) {
                        bindings.insert(Input::Button(btn), action);
                        continue;
                    }
                }

                if let Ok(deserializer) = toml::de::ValueDeserializer::parse(&key) {
                    if let Ok(key) = KeyCode::deserialize(deserializer) {
                        bindings.insert(Input::Key(key), action);
                        continue;
                    }
                }

                warn!("Unknown key: {}", key);
            }
        }
    } else {
        warn!("Unable to serialize bindings!");
    };
    bindings
}
