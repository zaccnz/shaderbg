/*
 * the menu file helps the app create and maintain both the window and
 * tray menus.
 */

use std::collections::HashMap;
use tao::{
    event::Event,
    menu::{ContextMenu, MenuBar, MenuId, MenuItem, MenuItemAttributes as MenuButton, MenuType},
};

use super::{AppEvent, AppState, WindowEvent, Windows};

enum Menu {
    MenuBar(MenuBar),
    ContextMenu(ContextMenu),
}

fn menu_new(menu_type: MenuType) -> Menu {
    match menu_type {
        MenuType::MenuBar => Menu::MenuBar(MenuBar::new()),
        MenuType::ContextMenu => Menu::ContextMenu(ContextMenu::new()),
        _ => panic!("Unknown menu type {:?}", menu_type),
    }
}

fn menu_add(menu: &mut Menu, item: MenuButton) -> MenuId {
    match menu {
        Menu::MenuBar(menu) => menu.add_item(item).id(),
        Menu::ContextMenu(menu) => menu.add_item(item).id(),
    }
}

fn menu_add_native(menu: &mut Menu, item: MenuItem) {
    match menu {
        Menu::MenuBar(menu) => menu.add_native_item(item),
        Menu::ContextMenu(menu) => menu.add_native_item(item),
    };
}

fn menu_unwrap_menubar(menu: Menu) -> MenuBar {
    match menu {
        Menu::MenuBar(menu) => menu,
        Menu::ContextMenu(_) => panic!("Tried to unwrap EitherMenu(ContextMenu) as MenuBar"),
    }
}

fn menu_unwrap_contextmenu(menu: Menu) -> ContextMenu {
    match menu {
        Menu::ContextMenu(menu) => menu,
        Menu::MenuBar(_) => panic!("Tried to unwrap EitherMenu(MenuBar) as ContextMenu"),
    }
}

pub struct MenuBuilder {
    app_state: AppState,

    items_window: HashMap<MenuId, fn(&Self, MenuType, MenuId)>,
    items_tray: HashMap<MenuId, fn(&Self, MenuType, MenuId)>,
    data_window: HashMap<MenuId, String>,
    data_tray: HashMap<MenuId, String>,
}

impl MenuBuilder {
    pub fn new(app_state: AppState) -> MenuBuilder {
        MenuBuilder {
            app_state,
            items_window: HashMap::new(),
            items_tray: HashMap::new(),
            data_window: HashMap::new(),
            data_tray: HashMap::new(),
        }
    }

    fn open_window(&self, window: Windows) {
        self.app_state
            .send(AppEvent::Window(WindowEvent::OpenUiWindow(window)))
            .unwrap();
    }

    fn build_recent_menu(&mut self, menu_type: MenuType) -> Menu {
        let mut menu = menu_new(menu_type);

        let state = self.app_state.get();
        for recent_scene in state.config.recent_scenes.iter() {
            if let Some(scene) = state.get_scene(recent_scene.scene.clone()) {
                let id = menu_add(
                    &mut menu,
                    MenuButton::new(
                        format!(
                            "{} ({})",
                            scene.descriptor.meta.name, scene.descriptor.meta.version
                        )
                        .as_str(),
                    ),
                );

                let items = match menu_type {
                    MenuType::ContextMenu => &mut self.items_tray,
                    MenuType::MenuBar => &mut self.items_window,
                    _ => panic!("Unknown menu type {:?}", menu_type),
                };
                let data = match menu_type {
                    MenuType::ContextMenu => &mut self.data_tray,
                    MenuType::MenuBar => &mut self.data_window,
                    _ => panic!("Unknown menu type {:?}", menu_type),
                };

                data.insert(id.clone(), recent_scene.scene.clone());

                items.insert(id, |menu, menu_type, id| {
                    let data = match menu_type {
                        MenuType::ContextMenu => &menu.data_tray,
                        MenuType::MenuBar => &menu.data_window,
                        _ => panic!("Unknown menu type {:?}", menu_type),
                    };

                    if let Some(scene_name) = data.get(&id) {
                        menu.app_state
                            .send(AppEvent::SetScene(scene_name.clone()))
                            .unwrap();
                    } else {
                        eprintln!("Recent scene {:?} did not store data", id);
                    }
                });
            }
        }

        menu
    }

    fn build_windows_menu(&mut self, menu_type: MenuType) -> Menu {
        let mut menu = menu_new(menu_type);
        let items = match menu_type {
            MenuType::MenuBar => &mut self.items_window,
            MenuType::ContextMenu => &mut self.items_tray,
            _ => todo!(),
        };

        let scene_browser_id = menu_add(&mut menu, MenuButton::new("Scene Browser"));
        items.insert(scene_browser_id, |menu, _, _| {
            menu.open_window(Windows::SceneBrowser)
        });
        let scene_settings_id = menu_add(&mut menu, MenuButton::new("Scene Settings"));
        items.insert(scene_settings_id, |menu, _, _| {
            menu.open_window(Windows::SceneSettings)
        });
        menu_add_native(&mut menu, MenuItem::Separator);
        let settigns_id = menu_add(&mut menu, MenuButton::new("Settings"));
        items.insert(settigns_id, |menu, _, _| {
            menu.open_window(Windows::Settings);
        });
        let performance_id = menu_add(&mut menu, MenuButton::new("Performance"));
        items.insert(performance_id, |menu, _, _| {
            menu.open_window(Windows::Performance)
        });

        menu
    }

    fn add_scene_menu(&mut self, mut menu: Menu) -> Menu {
        if let Some(scene) = self.app_state.get().scene() {
            menu_add(
                &mut menu,
                MenuButton::new(
                    format!("Current Scene: {}", scene.descriptor.meta.name.clone()).as_str(),
                )
                .with_enabled(false),
            );
            let pause_id = menu_add(&mut menu, MenuButton::new("Pause"));
            self.items_window.insert(pause_id, |_, _, _| {});
            let reload_id = menu_add(&mut menu, MenuButton::new("Reload"));
            self.items_window.insert(reload_id, |_, _, _| {});
        } else {
            menu_add(
                &mut menu,
                MenuButton::new(format!("No Scene Loaded",).as_str()).with_enabled(false),
            );
        }

        menu
    }

    fn build_window_app_menu(&mut self) -> MenuBar {
        let mut app_menu = MenuBar::new();
        app_menu.add_native_item(MenuItem::About(
            "shaderbg".into(),
            tao::menu::AboutMetadata {
                version: Some("1.0.0".into()),
                ..Default::default()
            },
        ));
        let settings_id = app_menu.add_item(MenuButton::new("Settings")).id();
        self.items_window.insert(settings_id, |menu, _, _| {
            menu.open_window(Windows::Settings)
        });
        app_menu.add_native_item(MenuItem::Separator);
        app_menu.add_native_item(MenuItem::Hide);
        app_menu.add_native_item(MenuItem::HideOthers);
        app_menu.add_native_item(MenuItem::Separator);
        app_menu.add_native_item(MenuItem::Quit);

        app_menu
    }

    fn build_window_scene_menu(&mut self) -> MenuBar {
        let mut scene_menu = MenuBar::new();

        scene_menu = menu_unwrap_menubar(self.add_scene_menu(Menu::MenuBar(scene_menu)));

        scene_menu.add_native_item(MenuItem::Separator);
        // possible quick settings for changing how the background is rendered
        let recent_scenes = menu_unwrap_menubar(self.build_recent_menu(MenuType::MenuBar));
        scene_menu.add_submenu("Recent", true, recent_scenes);

        scene_menu
    }

    fn build_window_help_menu(&mut self) -> MenuBar {
        let mut help_menu = MenuBar::new();
        let github_id = help_menu
            .add_item(MenuButton::new("GitHub Documentation..."))
            .id();
        self.items_window.insert(github_id, |_, _, _| {
            if let Err(err) = webbrowser::open("https://github.com/zaccnz/shaderbg") {
                eprintln!("Error opening GitHub page {:?}", err);
            }
        });

        help_menu
    }

    pub fn build_window_menu(&mut self) -> MenuBar {
        self.items_window.clear();

        let mut menu = MenuBar::new();
        menu.add_submenu("&shaderbg", true, self.build_window_app_menu());
        menu.add_submenu("&Scene", true, self.build_window_scene_menu());
        menu.add_submenu(
            "&Window",
            true,
            menu_unwrap_menubar(self.build_windows_menu(MenuType::MenuBar)),
        );
        menu.add_submenu("&Help", true, self.build_window_help_menu());
        menu
    }

    pub fn build_tray_menu(&mut self) -> ContextMenu {
        self.items_tray.clear();

        let mut menu = ContextMenu::new();

        let open_id = menu.add_item(MenuButton::new("Open")).id();
        self.items_tray.insert(open_id, |menu, _, _| {
            menu.app_state
                .send(AppEvent::Window(WindowEvent::StartWindow))
                .unwrap();
        });

        let windows_menu = menu_unwrap_contextmenu(self.build_windows_menu(MenuType::ContextMenu));
        menu.add_submenu("Open Window", true, windows_menu);

        menu.add_native_item(MenuItem::Separator);

        menu = menu_unwrap_contextmenu(self.add_scene_menu(Menu::ContextMenu(menu)));

        menu.add_native_item(MenuItem::Separator);

        let recent_scenes_menu =
            menu_unwrap_contextmenu(self.build_recent_menu(MenuType::ContextMenu));
        menu.add_submenu("Recent Scene", true, recent_scenes_menu);

        menu.add_native_item(MenuItem::Separator);

        let stop_background_id = menu.add_item(MenuButton::new("Stop Background")).id();
        self.items_tray.insert(stop_background_id, |_, _, _| {});
        let quit_id = menu.add_item(MenuButton::new("Quit")).id();
        self.items_tray.insert(quit_id, |menu, _, _| {
            menu.app_state
                .send(AppEvent::Window(WindowEvent::Quit))
                .unwrap();
        });

        menu
    }

    pub fn handle_event(&self, event: Event<WindowEvent>) {
        let handler = match event {
            Event::MenuEvent {
                menu_id,
                origin: MenuType::MenuBar,
                ..
            } => {
                let handler = self
                    .items_window
                    .get(&menu_id)
                    .map(|handler| (handler, MenuType::MenuBar, menu_id));

                if handler.is_none() {
                    println!(
                        "Menu handler received unknown menu id {:?} (origin: window)",
                        menu_id
                    );
                }

                handler
            }
            Event::MenuEvent {
                menu_id,
                origin: MenuType::ContextMenu,
                ..
            } => {
                let handler = self
                    .items_tray
                    .get(&menu_id)
                    .map(|handler| (handler, MenuType::ContextMenu, menu_id));

                if handler.is_none() {
                    println!(
                        "Menu handler received unknown menu id {:?} (origin: tray)",
                        menu_id
                    );
                }

                handler
            }
            _ => {
                println!("Menu handler received unknown event {:?}", event);
                None
            }
        };

        if let Some((handler, menu_type, menu_id)) = handler {
            handler(self, menu_type, menu_id);
        }
    }
}
