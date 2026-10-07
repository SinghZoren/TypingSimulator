use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use serde::{Deserialize, Serialize};

pub const KEYS: &[(&str, Code)] = &[
    ("F1", Code::F1),
    ("F2", Code::F2),
    ("F3", Code::F3),
    ("F4", Code::F4),
    ("F5", Code::F5),
    ("F6", Code::F6),
    ("F7", Code::F7),
    ("F8", Code::F8),
    ("F9", Code::F9),
    ("F10", Code::F10),
    ("F11", Code::F11),
    ("F12", Code::F12),
    ("A", Code::KeyA),
    ("B", Code::KeyB),
    ("C", Code::KeyC),
    ("D", Code::KeyD),
    ("E", Code::KeyE),
    ("F", Code::KeyF),
    ("G", Code::KeyG),
    ("H", Code::KeyH),
    ("I", Code::KeyI),
    ("J", Code::KeyJ),
    ("K", Code::KeyK),
    ("L", Code::KeyL),
    ("M", Code::KeyM),
    ("N", Code::KeyN),
    ("O", Code::KeyO),
    ("P", Code::KeyP),
    ("Q", Code::KeyQ),
    ("R", Code::KeyR),
    ("S", Code::KeyS),
    ("T", Code::KeyT),
    ("U", Code::KeyU),
    ("V", Code::KeyV),
    ("W", Code::KeyW),
    ("X", Code::KeyX),
    ("Y", Code::KeyY),
    ("Z", Code::KeyZ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub key_index: usize,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            key_index: 5,
            control: false,
            alt: false,
            shift: false,
            super_key: false,
        }
    }
}

impl ShortcutConfig {
    pub fn hotkey(self) -> Result<HotKey, &'static str> {
        let (_, code) = KEYS.get(self.key_index).ok_or("Select a shortcut key.")?;
        if self.key_index >= 12 && !(self.control || self.alt || self.super_key) {
            return Err(
                "Letter shortcuts need Ctrl, Alt, or Win/Cmd so ordinary typing still works.",
            );
        }
        let mut modifiers = Modifiers::empty();
        if self.control {
            modifiers |= Modifiers::CONTROL;
        }
        if self.alt {
            modifiers |= Modifiers::ALT;
        }
        if self.shift {
            modifiers |= Modifiers::SHIFT;
        }
        if self.super_key {
            modifiers |= Modifiers::SUPER;
        }
        Ok(HotKey::new(Some(modifiers), *code))
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.control {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push(if cfg!(target_os = "macos") {
                "Option"
            } else {
                "Alt"
            });
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.super_key {
            parts.push(if cfg!(target_os = "macos") {
                "Cmd"
            } else {
                "Win"
            });
        }
        parts.push(KEYS.get(self.key_index).map_or("?", |entry| entry.0));
        parts.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_letter_cannot_block_normal_typing() {
        let config = ShortcutConfig {
            key_index: 12,
            ..ShortcutConfig::default()
        };
        assert!(config.hotkey().is_err());
        assert!(ShortcutConfig {
            control: true,
            ..config
        }
        .hotkey()
        .is_ok());
    }
}
