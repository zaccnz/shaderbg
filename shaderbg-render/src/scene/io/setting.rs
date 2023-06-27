use hex_color::{HexColor, ParseHexColorError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Setting {
    Float {
        label: String,
        value: f32,
        min: f32,
        max: f32,
    },
    Colour3 {
        label: String,
        value: String,
    },
}

#[derive(Debug)]
pub enum SettingParseError {
    InvalidHex(ParseHexColorError),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SettingValue {
    Float(f32),
    Colour3([f32; 3]),
}

impl SettingValue {
    pub fn from_descriptor(setting: &Setting) -> Result<SettingValue, SettingParseError> {
        match setting {
            Setting::Float { value, .. } => Ok(SettingValue::Float(*value)),
            Setting::Colour3 { value, .. } => {
                let colour = match HexColor::parse(value.as_str()) {
                    Ok(colour) => colour,
                    Err(error) => return Err(SettingParseError::InvalidHex(error)),
                };

                Ok(SettingValue::Colour3([
                    (colour.r as f32) / 255.0,
                    (colour.g as f32) / 255.0,
                    (colour.b as f32) / 255.0,
                ]))
            }
        }
    }

    pub fn size(&self) -> usize {
        match self {
            SettingValue::Float(_) => 4,
            SettingValue::Colour3(_) => 12,
        }
    }

    pub fn alignment(&self) -> usize {
        match self {
            SettingValue::Float(_) => 4,
            SettingValue::Colour3(_) => 16,
        }
    }

    pub fn write(&self, buffer: &mut [u8]) {
        match self {
            SettingValue::Float(value) => {
                let bytes = bytemuck::bytes_of(value);
                for i in 0..self.size() {
                    buffer[i] = bytes[i];
                }
            }
            SettingValue::Colour3(value) => {
                let bytes = bytemuck::bytes_of(value);
                for i in 0..self.size() {
                    buffer[i] = bytes[i];
                }
            }
        }
    }
}
