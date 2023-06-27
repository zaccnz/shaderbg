use std::{collections::HashMap, io::Read, path::PathBuf};

use shaderbg_render::scene::{Scene, SceneError, Settings};

pub struct SceneEntry {
    pub name: Box<str>,
    pub scene: Scene,
}

fn load_scene_from_zip(path: PathBuf, settings_dir: PathBuf) -> Result<Scene, SceneError> {
    let file = match std::fs::File::open(path.clone()) {
        Ok(file) => file,
        Err(error) => {
            return Err(SceneError::ArchiveError(format!(
                "Failed to open zip archive {:?}",
                error
            )));
        }
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(error) => {
            return Err(SceneError::ArchiveError(format!(
                "Failed to open zip archive {:?}",
                error
            )));
        }
    };

    let scene_toml = {
        let scene_tomls: Vec<&str> = archive
            .file_names()
            .filter(|name| name.ends_with("/scene.toml") || name.eq(&"scene.toml"))
            .collect();

        if scene_tomls.is_empty() {
            return Err(SceneError::ArchiveError(format!(
                "Zip archive {:?} contains no scene.toml",
                path
            )));
        } else if scene_tomls.len() > 1 {
            return Err(SceneError::ArchiveError(format!(
                "Zip archive {:?} contains multiple scene.toml files",
                path
            )));
        } else {
            scene_tomls[0].to_owned()
        }
    };

    let mut scene_toml_contents: Vec<u8> = Vec::new();
    archive
        .by_name(&scene_toml)
        .unwrap()
        .read_to_end(&mut scene_toml_contents)
        .unwrap();

    let scene_root = if scene_toml.eq("scene.toml") {
        "".to_string()
    } else {
        scene_toml.replace("scene.toml", "")
    };

    let mut files: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        let path = match file.enclosed_name() {
            Some(path) => path.to_str().unwrap(),
            None => continue,
        };

        let destination_path = path.replace(&scene_root, "");

        let mut file_contents: Vec<u8> = Vec::new();
        file.read_to_end(&mut file_contents).unwrap();

        files.insert(destination_path, file_contents);
    }

    match Scene::load_from_memory(scene_toml_contents, files) {
        Ok(mut scene) => {
            let dir = settings_dir.join(format!(
                "{}.toml",
                path.file_name().unwrap().to_str().unwrap()
            ));
            if let Some(settings) = Settings::load(dir, &scene.descriptor) {
                scene.settings = settings;
            }
            Ok(scene)
        }
        Err(error) => Err(error),
    }
}

pub fn load_scenes(scene_dir: PathBuf, settings_dir: PathBuf) -> Box<[SceneEntry]> {
    println!("Loading scenes...");
    let scene_dir_iter = match std::fs::read_dir(scene_dir.clone()) {
        Ok(scene_dir_iter) => scene_dir_iter,
        Err(e) => panic!("{:?}", e),
    };

    let mut scenes = Vec::new();

    for scene_path in scene_dir_iter {
        let path = match scene_path {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Dir entry error: {:?}", e);
                continue;
            }
        };
        let path_name = path.file_name();
        let name = match path_name.to_str() {
            Some(name) => name,
            None => {
                eprint!("Scene {:?} has non-ascii name", path_name);
                continue;
            }
        };
        let scene = if name.ends_with(".zip") {
            match load_scene_from_zip(path.path(), settings_dir.clone()) {
                Ok(scene) => scene,
                Err(e) => {
                    eprintln!("Failed to load scene from archive {:?}: {:?}", path, e);
                    continue;
                }
            }
        } else if path.file_type().unwrap().is_dir() {
            match Scene::load(name.to_string(), scene_dir.clone(), settings_dir.clone()) {
                Ok(scene) => scene,
                Err(e) => {
                    eprintln!("Failed to load scene {:?}: {:?}", path, e);
                    continue;
                }
            }
        } else {
            continue;
        };

        scenes.push(SceneEntry {
            name: name.to_string().into_boxed_str(),
            scene,
        });
    }

    println!("Loaded {} scene(s)", scenes.len());
    scenes.into_boxed_slice()
}
