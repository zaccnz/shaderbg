use std::path::PathBuf;

use shaderbg_render::scene::Scene;

pub struct SceneEntry {
    pub name: Box<str>,
    pub scene: Scene,
}

pub fn load_scenes(scene_dir: PathBuf, settings_dir: PathBuf) -> Box<[SceneEntry]> {
    println!("scene dir {:?}", scene_dir);
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
        let scene = match Scene::load(name.to_string(), scene_dir.clone(), settings_dir.clone()) {
            Ok(scene) => scene,
            Err(e) => {
                eprintln!("Failed to load scene {:?}: {:?}", path, e);
                continue;
            }
        };

        scenes.push(SceneEntry {
            name: name.to_string().into_boxed_str(),
            scene,
        });
    }

    scenes.into_boxed_slice()
}
