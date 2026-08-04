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

#[derive(Default)]
pub struct ClientActions {
    bindings: FxHashMap<Input, String>,
}

impl ClientActions {
    pub fn load(&mut self, source: &HashMap<String, String>) {
        self.bindings = parse_bindings(source);
    }

    pub fn get_from_event(&mut self, event: &DeviceEvent) -> Option<&String> {
        match event {
            DeviceEvent::Key(raw_key_event) => match raw_key_event.physical_key {
                PhysicalKey::Code(key_code) if raw_key_event.state == ElementState::Pressed => {
                    let key = Input::Key(key_code);

                    return self.bindings.get(&key);
                }
                _ => {}
            },
            DeviceEvent::Button { button, state } if *state == ElementState::Pressed => {
                if let Ok(button) = MouseButton::try_from(*button) {
                    let key = Input::Button(button);

                    return self.bindings.get(&key);
                }
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

                let key = format!("\"{}\"", key.trim());
                let action = action.to_string();

                if let Ok(deserializer) = toml::de::ValueDeserializer::parse(&key) {
                    if let Ok(key) = KeyCode::deserialize(deserializer) {
                        bindings.insert(Input::Key(key), action);
                        continue;
                    }
                }

                if let Ok(button) = MouseButton::from_arg_value(&key) {
                    bindings.insert(Input::Button(button), action);
                }
            }
        }
    } else {
        warn!("Unable to serialize bindings!");
    };
    bindings
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

use serde::Deserialize;
use winit::keyboard::KeyCode;

use crate::client::cl_game_actions::Input;

    #[test]
    fn test() {
        // let mut map: HashMap<String, String> = HashMap::default();
        // map.insert(format!("{:?}", Input::Key(KeyCode::ArrowRight)), "jump".to_string());
        // let str = toml::to_string(&map).unwrap();
        // dbg!(str);

        let key = "\"KeyS\"";
        let deserializer = toml::de::ValueDeserializer::parse(key).unwrap();
        let key = KeyCode::deserialize(deserializer).unwrap();
        dbg!(key);
    }
}