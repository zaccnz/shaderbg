use std::{collections::HashMap, fs, io, path::PathBuf};

use crate::io::scene::{resource::Resource, Descriptor};

mod resources;
mod settings;
pub use resources::*;
pub use settings::*;

#[derive(Debug)]
pub enum SceneError {
    SceneTomlError(io::Error),
    InvalidResource {
        kind: String,
        id: String,
        error: String,
    },
    SettingsError(SettingParseError),
}

pub struct Scene {
    pub descriptor: Descriptor,
    pub settings: Settings,
    pub files: HashMap<String, Vec<u8>>,
}

impl Scene {
    pub fn load(path: PathBuf) -> Result<Scene, SceneError> {
        let toml_path = path.join("scene.toml");
        let toml_content = match fs::read(toml_path.as_path()) {
            Ok(content) => content,
            Err(error) => return Err(SceneError::SceneTomlError(error)),
        };

        let toml_string = match std::str::from_utf8(toml_content.as_slice()) {
            Ok(string) => string,
            Err(error) => {
                return Err(SceneError::SceneTomlError(io::Error::new(
                    io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let descriptor: Descriptor = match toml::from_str(toml_string) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                return Err(SceneError::SceneTomlError(io::Error::new(
                    io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let mut files = HashMap::new();

        for (id, resource) in descriptor.resources.iter() {
            match resource {
                Resource::Shader { src, .. } => {
                    let path = path.join(src);
                    let content = match fs::read(path) {
                        Ok(content) => content,
                        Err(error) => {
                            return Err(SceneError::InvalidResource {
                                kind: "Shader".to_string(),
                                id: id.clone(),
                                error: error.to_string(),
                            })
                        }
                    };

                    files.insert(id.clone(), content);
                }
                _ => {}
            }
        }

        let settings = match Settings::new(&descriptor) {
            Ok(settings) => settings,
            Err(error) => return Err(SceneError::SettingsError(error)),
        };

        Ok(Scene {
            descriptor,
            files,
            settings,
        })
    }

    pub fn load_from_memory(
        scene_toml: Vec<u8>,
        files: HashMap<String, Vec<u8>>,
    ) -> Result<Scene, SceneError> {
        let toml_string = match std::str::from_utf8(scene_toml.as_slice()) {
            Ok(string) => string,
            Err(error) => {
                return Err(SceneError::SceneTomlError(io::Error::new(
                    io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let descriptor: Descriptor = match toml::from_str(toml_string) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                return Err(SceneError::SceneTomlError(io::Error::new(
                    io::ErrorKind::Other,
                    error,
                )))
            }
        };

        for (id, resource) in descriptor.resources.iter() {
            match resource {
                Resource::Shader { .. } => {
                    if !files.contains_key(id) {
                        return Err(SceneError::InvalidResource {
                            kind: "Shader".to_string(),
                            id: id.clone(),
                            error: "file not provided".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        let settings = match Settings::new(&descriptor) {
            Ok(settings) => settings,
            Err(error) => return Err(SceneError::SettingsError(error)),
        };

        Ok(Scene {
            descriptor,
            files,
            settings,
        })
    }
}
