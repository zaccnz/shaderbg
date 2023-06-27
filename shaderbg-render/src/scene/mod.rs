use std::{collections::HashMap, fs, path::PathBuf};

use io::{resource::Resource, setting::SettingParseError, Descriptor};

pub mod io;
mod resources;
mod settings;
pub use resources::*;
pub use settings::*;

#[derive(Debug)]
pub enum SceneError {
    SceneTomlError(std::io::Error),
    InvalidResource {
        kind: String,
        id: String,
        error: String,
    },
    SettingsError(SettingParseError),
    ArchiveError(String),
}

pub struct Scene {
    pub descriptor: Descriptor,
    pub settings: Settings,
    pub files: HashMap<String, Vec<u8>>,
}

impl Scene {
    pub fn load(
        name: String,
        scene_dir: PathBuf,
        settings_dir: PathBuf,
    ) -> Result<Scene, SceneError> {
        let toml_path = scene_dir.join(name.clone()).join("scene.toml");
        let toml_content = match fs::read(toml_path.as_path()) {
            Ok(content) => content,
            Err(error) => return Err(SceneError::SceneTomlError(error)),
        };

        let toml_string = match std::str::from_utf8(toml_content.as_slice()) {
            Ok(string) => string,
            Err(error) => {
                return Err(SceneError::SceneTomlError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let descriptor: Descriptor = match toml::from_str(toml_string) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                return Err(SceneError::SceneTomlError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let mut files = HashMap::new();

        for (id, resource) in descriptor.resources.iter() {
            match resource {
                Resource::Shader { src, .. } | Resource::ShaderToy { src, .. } => {
                    let path = scene_dir.join(name.clone()).join(src);
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

        let settings_path = settings_dir.join(format!("{}.toml", name));
        let settings = if let Some(settings) = Settings::load(settings_path.clone(), &descriptor) {
            settings
        } else {
            match Settings::new(&descriptor) {
                Ok(settings) => settings,
                Err(error) => return Err(SceneError::SettingsError(error)),
            }
        };

        Ok(Scene {
            descriptor,
            files,
            settings,
        })
    }

    pub fn load_from_memory(
        scene_toml: Vec<u8>,
        mut virtual_folder: HashMap<String, Vec<u8>>,
    ) -> Result<Scene, SceneError> {
        let toml_string = match std::str::from_utf8(scene_toml.as_slice()) {
            Ok(string) => string,
            Err(error) => {
                return Err(SceneError::SceneTomlError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let descriptor: Descriptor = match toml::from_str(toml_string) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                return Err(SceneError::SceneTomlError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error,
                )))
            }
        };

        let mut files = HashMap::new();

        for (id, resource) in descriptor.resources.iter() {
            match resource {
                Resource::Shader { src, .. } | Resource::ShaderToy { src, .. } => {
                    if let Some(file) = virtual_folder.remove(src) {
                        files.insert(id.clone(), file);
                    } else {
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
