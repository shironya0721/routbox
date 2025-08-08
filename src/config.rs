use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Deserialize, Clone, Debug)]
pub struct KeyMap {
    pub stateful: HashMap<String, String>,
    pub stateless: HashMap<String, String>,
}

#[derive(Deserialize, Clone, Debug, Copy)]
pub enum KeyTriggerTiming {
    #[serde(rename = "on_press")]
    OnPress,
    #[serde(rename = "on_hold")]
    OnHold,
    #[serde(rename = "on_release")]
    OnRelease,
}

#[derive(Deserialize, Clone, Debug)]
pub struct KeyMappingConfig {
    pub keys: String,
    pub action: String,
    pub trigger: KeyTriggerTiming,
}

// In src/config.rs

// Add this module. It can go near the top with the `use` statements.
mod hex_serde {
    use serde::{self, Deserialize, Deserializer};

    // The custom deserialization function
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u16, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the field as a string first
        let s = String::deserialize(deserializer)?;
        // Ensure the string starts with "0x" and then parse the rest as a hex number
        if s.starts_with("0x") {
            u16::from_str_radix(&s[2..], 16).map_err(serde::de::Error::custom)
        } else {
            // If it doesn't start with "0x", return an error
            Err(serde::de::Error::custom(
                "expected a hex string starting with '0x'",
            ))
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum TourBoxDevice {
    #[serde(rename = "winusb")]
    WinUsb {
        #[serde(with = "hex_serde")]
        vid: u16,
        #[serde(with = "hex_serde")]
        pid: u16,
    },
    #[serde(rename = "serial")]
    Serial { serial_port: String, baud_rate: u32 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub device: TourBoxDevice,
    pub key_map: KeyMap,
    pub mappings: Vec<KeyMappingConfig>,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        let config = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(config)
    }
}
