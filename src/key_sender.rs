use std::collections::HashSet;

use enigo::{Axis, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use log::info;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum TourAction {
    KeyPress(String),
    KeyClick(String),
    KeyRelease(String),
    UiAction(String)
}

#[derive(Debug)]
pub struct KeySender {
    enigo: Enigo,
    active_key: HashSet<Key>,
}

#[derive(Error, Debug)]
pub enum KeySenderError {
    #[error("the key `{0}` is not available")]
    UnknownKey(String),
}

impl KeySender {
    pub fn new() -> Self {
        let enigo = Enigo::new(&Settings::default()).unwrap();
        Self {
            enigo,
            active_key: HashSet::new(),
        }
    }

    fn parse_key(key_str: &str) -> Result<Key, KeySenderError> {
        let uppercase_key = key_str.to_uppercase();
        match uppercase_key.as_str() {
            // Modifiers
            "ALT" | "ALT_L" | "ALT_R" => Ok(Key::Alt),
            "CONTROL" | "CTRL" | "CTRL_L" | "CTRL_R" => Ok(Key::Control),
            "SHIFT" | "SHIFT_L" | "SHIFT_R" => Ok(Key::Shift),
            "WIN" | "WIN_L" | "WIN_R" | "SUPER" | "COMMAND" => Ok(Key::Meta),

            // Arrow keys
            "DOWN" | "DOWN_ARROW" => Ok(Key::DownArrow),
            "LEFT" | "LEFT_ARROW" => Ok(Key::LeftArrow),
            "RIGHT" | "RIGHT_ARROW" => Ok(Key::RightArrow),
            "UP" | "UP_ARROW" => Ok(Key::UpArrow),

            // Special keys
            "BACKSPACE" => Ok(Key::Backspace),
            "CAPSLOCK" => Ok(Key::CapsLock),
            "DELETE" => Ok(Key::Delete),
            "END" => Ok(Key::End),
            "ENTER" => Ok(Key::Return),
            "ESCAPE" => Ok(Key::Escape),
            "F1" => Ok(Key::F1),
            "F2" => Ok(Key::F2),
            "F3" => Ok(Key::F3),
            "F4" => Ok(Key::F4),
            "F5" => Ok(Key::F5),
            "F6" => Ok(Key::F6),
            "F7" => Ok(Key::F7),
            "F8" => Ok(Key::F8),
            "F9" => Ok(Key::F9),
            "F10" => Ok(Key::F10),
            "F11" => Ok(Key::F11),
            "F12" => Ok(Key::F12),
            "HOME" => Ok(Key::Home),
            "PAGEDOWN" => Ok(Key::PageDown),
            "PAGEUP" => Ok(Key::PageUp),
            "SPACE" => Ok(Key::Space),
            "TAB" => Ok(Key::Tab),

            "A" => Ok(Key::A),
            "B" => Ok(Key::B),
            "C" => Ok(Key::C),
            "D" => Ok(Key::D),
            "E" => Ok(Key::E),
            "F" => Ok(Key::F),
            "G" => Ok(Key::G),
            "H" => Ok(Key::H),
            "I" => Ok(Key::I),
            "J" => Ok(Key::J),
            "K" => Ok(Key::K),
            "L" => Ok(Key::L),
            "M" => Ok(Key::M),
            "N" => Ok(Key::N),
            "O" => Ok(Key::O),
            "P" => Ok(Key::P),
            "Q" => Ok(Key::Q),
            "R" => Ok(Key::R),
            "S" => Ok(Key::S),
            "T" => Ok(Key::T),
            "U" => Ok(Key::U),
            "V" => Ok(Key::V),
            "W" => Ok(Key::W),
            "X" => Ok(Key::X),
            "Y" => Ok(Key::Y),
            "Z" => Ok(Key::Z),

            // Special characters that don't require a shift modifier
            "-" => Ok(Key::Other(0xBD_u32)), // OEM_MINUS
            "=" => Ok(Key::Other(0xBB_u32)), // OEM_PLUS
            "[" => Ok(Key::Other(0xDB_u32)), // OEM_4
            "]" => Ok(Key::Other(0xDD_u32)), // OEM_6
            "" => Ok(Key::Other(0xDC_u32)),  // OEM_5
            ";" => Ok(Key::Other(0xBA_u32)), // OEM_1
            "'" => Ok(Key::Other(0xDE_u32)), // OEM_7
            "," => Ok(Key::Other(0xBC_u32)), // OEM_COMMA
            "." => Ok(Key::Other(0xBE_u32)), // OEM_PERIOD
            "/" => Ok(Key::Other(0xBF_u32)), // OEM_2
            "`" => Ok(Key::Other(0xC0_u32)), // OEM_3

            // @+number for special virtual key
            k if key_str.starts_with("@") => Ok(Key::Other(
                k.split("@")
                    .collect::<String>()
                    .parse()
                    .map_err(|_| KeySenderError::UnknownKey(k.to_string()))?,
            )),

            k => Err(KeySenderError::UnknownKey(k.to_string())), // No match found
        }
    }

    pub fn send_key(&mut self, action: &TourAction) -> Result<(), KeySenderError> {
        info!("send_key {action:?}");
        match action {
            TourAction::KeyPress(s) => {
                let key = KeySender::parse_key(s)?;
                self.active_key.insert(key);
                self.enigo.key(key, Direction::Press).unwrap();
            }
            TourAction::KeyClick(s) => match s.to_uppercase().as_str() {
                "WHEEL_UP" => {
                    self.enigo.scroll(-1, Axis::Vertical).unwrap();
                }
                "WHEEL_DOWN" => {
                    self.enigo.scroll(1, Axis::Vertical).unwrap();
                }
                _ => {
                    let mut to_be_release = Vec::with_capacity(10);
                    for k in s.split("+").into_iter() {
                        let key = KeySender::parse_key(k)?;
                        if !self.active_key.contains(&key) {
                            self.enigo.key(key, Direction::Press).unwrap();
                            to_be_release.push(key);
                        }
                    }
                    for key in to_be_release.into_iter().rev() {
                        self.enigo.key(key, Direction::Release).unwrap();
                    }
                }
            },
            TourAction::KeyRelease(s) => {
                let key = KeySender::parse_key(s)?;
                self.active_key.remove(&key);
                self.enigo.key(key, Direction::Release).unwrap();
            }
            _ => {
                // ignore other action
            }
        }

        Ok(())
    }
}
