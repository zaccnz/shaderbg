/*
 * System tray menu
 */
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopWindowTarget},
    menu::{ContextMenu, CustomMenuItem, MenuItemAttributes, MenuType},
    system_tray::{SystemTray, SystemTrayBuilder},
    TrayId,
};

#[cfg(target_os = "linux")]
use tao::platform::linux::SystemTrayBuilderExtLinux;

use crate::app::{AppEvent, AppState, WindowEvent};

pub struct Tray {
    system_tray: Option<SystemTray>,
    quit: CustomMenuItem,
    menu_item: CustomMenuItem,
    app_state: AppState,
}

impl Tray {
    pub fn build(event_loop: &EventLoopWindowTarget<WindowEvent>, app_state: AppState) -> Tray {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png");

        let main_tray_id = TrayId::new("main-tray");
        let icon = load_icon(std::path::Path::new(path));
        let mut tray_menu = ContextMenu::new();
        let menu_item = tray_menu.add_item(MenuItemAttributes::new("Open Window"));

        let quit = tray_menu.add_item(MenuItemAttributes::new("Quit"));

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

        Tray {
            system_tray,
            quit,
            menu_item,
            app_state,
        }
    }

    pub fn handle(
        &mut self,
        event: Event<WindowEvent>,
        _event_loop: &EventLoopWindowTarget<WindowEvent>,
        control_flow: &mut ControlFlow,
    ) {
        match event {
            Event::MenuEvent {
                menu_id,
                origin: MenuType::ContextMenu,
                ..
            } => {
                if menu_id == self.quit.clone().id() {
                    *control_flow = ControlFlow::Exit;
                } else if menu_id == self.menu_item.clone().id() {
                    self.app_state
                        .send(AppEvent::Window(WindowEvent::OpenWindow))
                        .unwrap();
                }
            }
            /*
            Event::TrayEvent {
                id,
                bounds,
                event,
                position,
                ..
            } => {
                let tray = if id == self.tray_id {
                    "main"
                } else {
                    "unknown"
                };
                println!(
                    "tray `{}` event: {:?} {:?} {:?}",
                    tray, event, bounds, position
                );
            }*/
            _ => (),
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
