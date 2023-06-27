use std::collections::HashMap;

use shaderbg_render::scene::Scene;

// to avoid having to use std::io on WASM, i am hardcoding all
// of the installed scenes
pub fn load() -> Vec<(String, Scene)> {
    let mut scenes = Vec::new();

    {
        let scene_toml = include_bytes!("../../scenes/waves/scene.toml").to_vec();
        let scene_files = HashMap::from([
            (
                "vertices.wgsl".to_string(),
                include_bytes!("../../scenes/waves/vertices.wgsl").to_vec(),
            ),
            (
                "waves.wgsl".to_string(),
                include_bytes!("../../scenes/waves/waves.wgsl").to_vec(),
            ),
        ]);
        let waves = match Scene::load_from_memory(scene_toml, scene_files) {
            Ok(scene) => scene,
            Err(e) => panic!("{:?}", e),
        };
        scenes.push(("waves".to_string(), waves));
    }

    {
        let scene_toml = include_bytes!("../../scenes/shadertoy-ltcGDl/scene.toml").to_vec();
        let scene_files = HashMap::from([(
            "desert.glsl".to_string(),
            include_bytes!("../../scenes/shadertoy-ltcGDl/desert.glsl").to_vec(),
        )]);
        let desert = match Scene::load_from_memory(scene_toml, scene_files) {
            Ok(scene) => scene,
            Err(e) => panic!("{:?}", e),
        };
        scenes.push(("desert".to_string(), desert));
    }

    {
        let scene_toml = include_bytes!("../../scenes/shadertoy-mdBSRt/scene.toml").to_vec();
        let scene_files = HashMap::from([(
            "tiles.glsl".to_string(),
            include_bytes!("../../scenes/shadertoy-mdBSRt/tiles.glsl").to_vec(),
        )]);
        let tiles = match Scene::load_from_memory(scene_toml, scene_files) {
            Ok(scene) => scene,
            Err(e) => panic!("{:?}", e),
        };
        scenes.push(("tiles".to_string(), tiles));
    }

    scenes
}
