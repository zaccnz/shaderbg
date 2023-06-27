use std::{
    collections::{hash_map::Iter, HashMap},
    fs,
    path::PathBuf,
};

use crate::scene::io::{
    setting::{SettingParseError, SettingValue},
    Descriptor,
};

pub struct Settings {
    data: HashMap<String, SettingValue>,
}

#[allow(dead_code)]
impl Settings {
    pub fn new(descriptor: &Descriptor) -> Result<Settings, SettingParseError> {
        let mut data = HashMap::new();

        for (key, setting) in descriptor.settings.iter() {
            let setting = SettingValue::from_descriptor(setting)?;
            data.insert(key.clone(), setting);
        }

        Ok(Settings { data })
    }

    pub fn load(path: PathBuf, descriptor: &Descriptor) -> Option<Settings> {
        let settings_string = if let Ok(settings) = fs::read_to_string(path.clone()) {
            settings
        } else {
            eprintln!("Scene settings {:?} not found, ignoring", path);
            return None;
        };

        let mut data: HashMap<String, SettingValue> = match toml::from_str(&settings_string) {
            Ok(data) => data,
            Err(error) => {
                eprintln!(
                    "Failed to parse scene settings file {:?}:\n{:#?}\nIgnoring",
                    path, error
                );
                return None;
            }
        };

        for (key, setting) in descriptor.settings.iter() {
            let setting = if let Ok(setting) = SettingValue::from_descriptor(setting) {
                setting
            } else {
                return None;
            };

            if let Some(value) = data.get(key) {
                if std::mem::discriminant(value) != std::mem::discriminant(&setting) {
                    eprintln!(
                        "Scene setting {}: {:?} does not match scene.toml's {:?}",
                        key, value, setting
                    );
                }
            } else {
                data.insert(key.clone(), setting);
            }
        }

        Some(Settings { data })
    }

    pub fn save(&self, path: PathBuf) -> Result<(), String> {
        let parent = if let Some(path) = path.parent() {
            path
        } else {
            panic!("Failed to find parent of {:?}", path);
        };

        if let Err(err) = fs::create_dir_all(parent) {
            panic!(
                "Failed to create settings directory {:?}, error: {:?}",
                err, parent
            );
        }

        let settings_string = match toml::to_string(&self.data) {
            Ok(str) => str,
            Err(e) => return Err(e.to_string()),
        };

        match fs::write(path, settings_string) {
            Ok(()) => Ok(()),
            Err(e) => return Err(e.to_string()),
        }
    }

    pub fn get(&self, key: &String) -> Option<&SettingValue> {
        self.data.get(key)
    }

    pub fn update(&mut self, key: &String, value: SettingValue) {
        *self.data.get_mut(key).unwrap() = value;
    }

    pub fn iter(&self) -> Iter<String, SettingValue> {
        self.data.iter()
    }

    pub fn clone(&self) -> Settings {
        let mut new_data = HashMap::new();
        new_data.extend(
            self.data
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        Settings { data: new_data }
    }

    pub fn reset(&mut self, descriptor: &Descriptor) -> Result<(), SettingParseError> {
        for (key, setting) in descriptor.settings.iter() {
            let setting = SettingValue::from_descriptor(setting)?;
            *self.data.get_mut(key).unwrap() = setting;
        }

        Ok(())
    }
}
