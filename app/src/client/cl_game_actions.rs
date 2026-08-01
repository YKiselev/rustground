use std::collections::HashMap;

use argh::FromArgValue;
use bitflags::{Flags, bitflags};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;
use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, FromArgValue)]
#[repr(u32)]
pub enum MouseButton {
    Mouse1 = 0,
    Mouse2 = 1,
    Mouse3 = 2,
}

impl TryFrom<u32> for MouseButton {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Mouse1),
            1 => Ok(Self::Mouse2),
            2 => Ok(Self::Mouse3),
            _ => Err("Invalid button!"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Input {
    Key(KeyCode),
    Button(MouseButton),
}

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
    bindings: FxHashMap<Input, GameActionFlags>,
}

impl GameActions {
    pub fn new(source: &HashMap<String, String>) -> Self {
        let bindings = parse_bindings(source);
        report_missing_bindings(&bindings);
        Self {
            flags: GameActionFlags::empty(),
            bindings,
        }
    }

    pub fn update_from_key(&mut self, key: PhysicalKey, state: ElementState) {
        match key {
            PhysicalKey::Code(key_code) => {
                let key = Input::Key(key_code);

                if let Some(&flag) = self.bindings.get(&key) {
                    match state {
                        ElementState::Pressed => self.flags.insert(flag),
                        ElementState::Released => self.flags.remove(flag),
                    }
                }
            }
            PhysicalKey::Unidentified(_) => {}
        }
    }

    pub fn update_from_button(&mut self, button: u32, state: ElementState) {
        if let Ok(button) = MouseButton::try_from(button) {
            let key = Input::Button(button);

            if let Some(&flag) = self.bindings.get(&key) {
                match state {
                    ElementState::Pressed => self.flags.insert(flag),
                    ElementState::Released => self.flags.remove(flag),
                }
            }
        }
    }
}

fn parse_bindings(source: &HashMap<String, String>) -> FxHashMap<Input, GameActionFlags> {
    let mut bindings = FxHashMap::default();
    if let Ok(str) = toml::to_string(&source) {
        for line in str.lines() {
            if let Some((action, key)) = line.split_once("=") {
                if let Ok(flag) =
                    bitflags::parser::from_str::<GameActionFlags>(&action.trim().to_uppercase())
                {
                    let trimmed = key.trim();
                    if let Ok(deserializer) = toml::de::ValueDeserializer::parse(trimmed) {
                        if let Ok(key) = KeyCode::deserialize(deserializer) {
                            bindings.insert(Input::Key(key), flag);
                            continue;
                        }
                    }
                    
                    if let Some(stripped) = trimmed
                        .strip_prefix("\"")
                        .and_then(|v| v.strip_suffix("\""))
                        .map(|v| v.to_lowercase())
                    {
                        if let Ok(button) = MouseButton::from_arg_value(&stripped) {
                            bindings.insert(Input::Button(button), flag);
                        }
                    }
                }
            } else {
                continue;
            }
        }
    } else {
        warn!("Unable to serialize bindings!");
    };
    bindings
}

fn report_missing_bindings(bindings: &FxHashMap<Input, GameActionFlags>) {
    let flags = bindings.values().map(|v| *v).collect::<GameActionFlags>();

    for (name, flag) in GameActionFlags::iter_defined_names() {
        if !flags.contains(flag) {
            warn!("Unbound action: {}", name);
        }
    }
}
