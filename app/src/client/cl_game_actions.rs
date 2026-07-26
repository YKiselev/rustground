use std::{collections::HashMap, ops::Index};

use bitflags::bitflags;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;
use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
    #[serde(transparent)]
    pub struct GameActionFlags: u32 {
        const FORWARD = 1 << 0;
        const BACKWARD = 1 << 1;
        const STRAFE_LEFT = 1 << 2;
        const STRAFE_RIGHT = 1 << 3;
        const JUMP = 1 << 4;
        const CROUCH = 1 << 5;
        const FIRE = 1 << 6;
        const SPRINT = 1 << 7;
        const LEFT = 1 << 8;
        const RIGHT = 1 << 9;
        const USE = 1 << 10;
    }
}

pub(crate) struct GameActions {
    flags: GameActionFlags,
    bindings: FxHashMap<KeyCode, GameActionFlags>,
}

impl GameActions {
    pub fn new(source: &HashMap<String, String>) -> Self {
        let bindings = parse_bindings(source);
        Self {
            flags: GameActionFlags::empty(),
            bindings,
        }
    }

    pub fn update(&mut self, key: PhysicalKey, state: ElementState) {
        match key {
            PhysicalKey::Code(key_code) => {
                if let Some(&flag) = self.bindings.get(&key_code) {
                    match state {
                        ElementState::Pressed => self.flags.insert(flag),
                        ElementState::Released => self.flags.remove(flag),
                    }
                }
            }
            PhysicalKey::Unidentified(_) => {}
        }
    }
}

fn parse_bindings(source: &HashMap<String, String>) -> FxHashMap<KeyCode, GameActionFlags> {
    let mut bindings = FxHashMap::default();
    if let Ok(str) = toml::to_string(&source) {
        for line in str.lines() {
            if let Some((action, key)) = line.split_once("=") {
                let line = format!("{}={}", action.trim().to_uppercase(), key.trim());

                match toml::from_str::<HashMap<GameActionFlags, KeyCode>>(&line) {
                    Ok(map) => {
                        for (flag, key) in map.into_iter() {
                            bindings.insert(key, flag);
                        }
                    }
                    Err(e) => warn!("Skipping line \"{}\": {}", &line, e.message()),
                }
            } else {
                continue;
            }
        }
    } else {
        warn!("Unable to serialize bindings!");
    };
    dbg!(&bindings);
    bindings
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Deserialize;
    use winit::keyboard::KeyCode;

    use crate::client::cl_game_actions::GameActionFlags;

    #[test]
    fn test() {
        let str = "\"Escape\"";
        let deserializer = toml::de::ValueDeserializer::parse(str).unwrap();
        let key = KeyCode::deserialize(deserializer).unwrap();
        println!("key: {:?}", key);

        let str = "\"FORWARD\"";
        let deserializer = toml::de::ValueDeserializer::parse(str).unwrap();
        let action = GameActionFlags::deserialize(deserializer).unwrap();
        println!("action: {:?}", action);

        let str = "\"Escape\" = \"JUMP\"";
        let map: HashMap<KeyCode, GameActionFlags> = toml::from_str(str).unwrap();
        println!("Map: {:?}", map);
    }
}
