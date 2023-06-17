/*
 * System tray menu
 */
use tao::{
    event_loop::EventLoopWindowTarget,
    system_tray::{SystemTray, SystemTrayBuilder},
    TrayId,
};

#[cfg(target_os = "linux")]
use tao::platform::linux::SystemTrayBuilderExtLinux;

use crate::app::{MenuBuilder, WindowEvent};

pub struct Tray {
    system_tray: Option<SystemTray>,
}

impl Tray {
    pub fn build(
        event_loop: &EventLoopWindowTarget<WindowEvent>,
        menu_builder: &mut MenuBuilder,
    ) -> Tray {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png");

        let main_tray_id = TrayId::new("main-tray");
        let icon = load_icon(std::path::Path::new(path));

        let tray_menu = menu_builder.build_tray_menu();

        #[cfg(target_os = "linux")]
        let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
            .with_id(main_tray_id)
            .with_temp_icon_dir(std::path::Path::new("/tmp/tao-examples"))
            .build(&event_loop)
            .unwrap();

        #[cfg(target_os = "windows")]
        let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
            .with_id(main_tray_id)
            .with_tooltip("tao - windowing creation library")
            .build(&event_loop)
            .unwrap();

        #[cfg(target_os = "macos")]
        let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
            .with_id(main_tray_id)
            .with_tooltip("tao - windowing creation library")
            .build(&event_loop)
            .unwrap();

        let system_tray = Some(system_tray);

        Tray { system_tray }
    }

    pub fn rebuild_menus(&mut self, menu_builder: &mut MenuBuilder) {
        if let Some(tray) = self.system_tray.as_mut() {
            let tray_menu = menu_builder.build_tray_menu();
            tray.set_menu(&tray_menu);
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.system_tray.take();
    }
}

fn load_icon(path: &std::path::Path) -> tao::system_tray::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tao::system_tray::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon")
}
