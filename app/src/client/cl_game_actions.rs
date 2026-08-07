use std::sync::atomic::{AtomicU32, Ordering};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Default)]
pub(crate) struct GameActions {
    flags: AtomicU32,
}

impl GameActions {
    pub fn insert(&self, bits: u32) -> u32 {
        self.flags.fetch_or(bits, Ordering::Relaxed)
    }

    pub fn remove(&self, bits: u32) -> u32 {
        self.flags.fetch_and(!bits, Ordering::Relaxed)
    }

    pub fn get(&self) -> GameActionFlags {
        GameActionFlags::from_bits_truncate(self.flags.load(Ordering::Relaxed))
    }
}
