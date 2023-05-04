// Helper to deserialize maps from TOML
use serde::Deserialize;
use std::{
    collections::{hash_map::Iter, HashMap},
    fmt::Debug,
};
use toml::map::Map;

#[derive(Clone)]
pub struct DeserializableMap<T> {
    data: HashMap<String, T>,
}

impl<T: Debug> Debug for DeserializableMap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

impl<T> DeserializableMap<T> {
    pub fn get(&self, key: &String) -> Option<&T> {
        self.data.get(key)
    }

    #[allow(dead_code)]
    pub fn contains_key(&self, key: &String) -> bool {
        self.data.contains_key(key)
    }

    pub fn iter(&self) -> Iter<'_, String, T> {
        self.data.iter()
    }
}

impl<'de, T> Deserialize<'de> for DeserializableMap<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = Map::deserialize(deserializer)?;

        let mut data: HashMap<String, T> = HashMap::new();
        for (key, value) in map.into_iter() {
            let value: T = match value.try_into() {
                Ok(value) => value,
                Err(error) => return Err(serde::de::Error::custom(error.to_string())),
            };

            data.insert(key.clone(), value);
        }
        Ok(DeserializableMap { data })
    }
}
