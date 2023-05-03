use std::collections::{hash_map::Iter, HashMap};

use hex_color::{HexColor, ParseHexColorError};

use crate::io::scene::{setting::Setting as DescriptorSetting, Descriptor};

#[derive(Debug)]
pub enum SettingParseError {
    InvalidHex(ParseHexColorError),
}

pub enum Setting {
    Float(f32),
    Colour3([f32; 3]),
}

impl Setting {
    pub fn from_descriptor(setting: &DescriptorSetting) -> Result<Setting, SettingParseError> {
        match setting {
            DescriptorSetting::Float { value, .. } => Ok(Setting::Float(*value)),
            DescriptorSetting::Colour3 { value, .. } => {
                let colour = match HexColor::parse(value.as_str()) {
                    Ok(colour) => colour,
                    Err(error) => return Err(SettingParseError::InvalidHex(error)),
                };

                Ok(Setting::Colour3([
                    (colour.r as f32) / 255.0,
                    (colour.g as f32) / 255.0,
                    (colour.b as f32) / 255.0,
                ]))
            }
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Setting::Float(_) => 4,
            Setting::Colour3(_) => 12,
        }
    }

    pub fn alignment(&self) -> usize {
        match self {
            Setting::Float(_) => 4,
            Setting::Colour3(_) => 16,
        }
    }

    pub fn write(&self, buffer: &mut [u8]) {
        match self {
            Setting::Float(value) => {
                let bytes = bytemuck::bytes_of(value);
                for i in 0..self.size() {
                    buffer[i] = bytes[i];
                }
            }
            Setting::Colour3(value) => {
                let bytes = bytemuck::bytes_of(value);
                for i in 0..self.size() {
                    buffer[i] = bytes[i];
                }
            }
        }
    }
}

pub struct Settings {
    data: HashMap<String, Setting>,
}

#[allow(dead_code)]
impl Settings {
    pub fn new(descriptor: &Descriptor) -> Result<Settings, SettingParseError> {
        let mut data = HashMap::new();

        for (key, setting) in descriptor.settings.iter() {
            let setting = Setting::from_descriptor(setting)?;
            data.insert(key.clone(), setting);
        }

        Ok(Settings { data })
    }

    pub fn get(&self, key: &String) -> Option<&Setting> {
        self.data.get(key)
    }

    pub fn update(&mut self, key: &String, value: Setting) {
        *self.data.get_mut(key).unwrap() = value;
    }

    pub fn iter(&self) -> Iter<String, Setting> {
        self.data.iter()
    }

    pub fn reset(&mut self, descriptor: &Descriptor) -> Result<(), SettingParseError> {
        for (key, setting) in descriptor.settings.iter() {
            let setting = Setting::from_descriptor(setting)?;
            *self.data.get_mut(key).unwrap() = setting;
        }

        Ok(())
    }
}
