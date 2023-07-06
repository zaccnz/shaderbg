use core::fmt::Debug;
use std::sync::mpsc;
use tao::{
    dpi::PhysicalSize,
    event::{self, Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    platform::windows::WindowBuilderExtWindows,
    window::{Window, WindowBuilder},
};

use crate::app::{AppEvent, AppState, ThreadEvent};
use shaderbg_render::{
    gfx::{buffer::ShaderToy, Gfx, GfxContext},
    scene::{io::setting::SettingValue, Resources},
};

#[derive(Debug)]
pub enum BackgroundEvent {
    TaoEvent(Event<'static, ThreadEvent>),
    SettingUpdated(String, SettingValue),
    SceneChanged,
    Stop,
}

/*
pub enum WindowId {
    Winit(tao::window::WindowId),
    HWND,
}

pub trait RawWindow:
    raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle + Send + Sync
{
    fn id(&self) -> WindowId;
    fn inner_size(&self) -> PhysicalSize<u32>;
    fn request_redraw(&self);
}

impl RawWindow for Window {
    fn id(&self) -> WindowId {
        WindowId::Winit(self.id())
    }

    fn inner_size(&self) -> PhysicalSize<u32> {
        self.inner_size()
    }

    fn request_redraw(&self) {
        self.request_redraw()
    }
}
*/
pub struct Background {
    pub window: Window,
    app_state: AppState,
    gfx_context: Option<GfxContext>,
}

impl Debug for Background {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Background").finish_non_exhaustive()
    }
}

/*
#[cfg(target_os = "windows")]
struct WindowsHandle {
    pub handle: raw_window_handle::RawWindowHandle,
    pub display_handle: raw_window_handle::RawDisplayHandle,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsHandle {}
#[cfg(target_os = "windows")]
unsafe impl Sync for WindowsHandle {}

#[cfg(target_os = "windows")]
impl RawWindow for WindowsHandle {
    fn id(&self) -> WindowId {
        WindowId::HWND
    }

    fn inner_size(&self) -> PhysicalSize<u32> {
        PhysicalSize {
            width: 1024,
            height: 576,
        }
    }

    fn request_redraw(&self) {}
}

#[cfg(target_os = "windows")]
unsafe impl raw_window_handle::HasRawWindowHandle for WindowsHandle {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        self.handle
    }
}

#[cfg(target_os = "windows")]
unsafe impl raw_window_handle::HasRawDisplayHandle for WindowsHandle {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        self.display_handle
    }
}
*/

#[cfg(target_os = "windows")]
fn get_background_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    use windows::{
        s, w,
        Win32::{
            Foundation::{BOOL, HWND, LPARAM},
            UI::WindowsAndMessaging::{
                EnumWindows, FindWindowA, FindWindowExA, FindWindowExW, SendMessageTimeoutA,
                SMTO_NORMAL,
            },
        },
    };

    unsafe {
        let progman = FindWindowA(s!("ProgMan"), None);

        SendMessageTimeoutA(progman, 0x052C, None, None, SMTO_NORMAL, 1000, None);

        unsafe extern "system" fn enum_windows_cb(hwnd: HWND, l_param: LPARAM) -> BOOL {
            let wallpaper = &mut *(l_param.0 as *mut HWND);
            let shelldll_defview = FindWindowExA(hwnd, None, s!("SHELLDLL_DefView"), None);
            if shelldll_defview.0 > 0 {
                *wallpaper = FindWindowExW(None, hwnd, w!("WorkerW"), None);
            }

            BOOL(1)
        }

        let mut wallpaper = HWND(0);
        EnumWindows(
            Some(enum_windows_cb),
            LPARAM(&mut wallpaper as *mut HWND as isize),
        );

        if wallpaper == HWND(0) {
            eprintln!("Failed to find HWND of wallpaper");
            return None;
        }

        Some(wallpaper)
    }
}

// modified from 'swizzleSendEvent'.
// https://github.com/maketechnology/cefrust/blob/6404c4dc0c984b3ca92fff7d42d7599cd432f088/cefrustlib/src/lib.rs#LL154C24-L154C24
// because i cant override the function, i swizzled it to un-constrain window
// placment.  this lets me place the window behind the menu bar. transparency
// is linked to the background image behind our wallpaper, so this is only
// important for expose where you could see the gap.  however, later, i may
// generate background images and set them at an interval.
#[cfg(target_os = "macos")]
fn swizzle_constrainframerect_toscreen() {
    use cocoa::{base::id, foundation::NSRect};
    use objc::runtime::{self, Class, Method, Object, Sel, NO};
    use objc::{Encode, EncodeArguments, Encoding};
    use std::ffi::CString;

    static mut HAS_SWIZZLED: bool = false;

    fn count_args(sel: Sel) -> usize {
        sel.name().chars().filter(|&c| c == ':').count()
    }

    fn method_type_encoding(ret: &Encoding, args: &[Encoding]) -> CString {
        let mut types = ret.as_str().to_owned();
        // First two arguments are always self and the selector
        types.push_str(<*mut Object>::encode().as_str());
        types.push_str(Sel::encode().as_str());
        types.extend(args.iter().map(|e| e.as_str()));
        CString::new(types).unwrap()
    }

    pub unsafe fn add_method<F>(cls: *mut Class, sel: Sel, func: F)
    where
        F: objc::declare::MethodImplementation<Callee = Object>,
    {
        let encs = F::Args::encodings();
        let encs = encs.as_ref();
        let sel_args = count_args(sel);
        assert!(
            sel_args == encs.len(),
            "Selector accepts {} arguments, but function accepts {}",
            sel_args,
            encs.len(),
        );

        let types = method_type_encoding(&F::Ret::encode(), encs);
        let success = runtime::class_addMethod(cls, sel, func.imp(), types.as_ptr());
        assert!(success != NO, "Failed to add method {:?}", sel);
    }

    let cls_nm = CString::new("NSWindow").unwrap();
    let cls = unsafe { runtime::objc_getClass(cls_nm.as_ptr()) as *mut Class };
    assert!(!cls.is_null(), "null class");

    #[allow(non_snake_case)]
    extern "C" fn swizzled_constrainFrameRect_toScreen_(
        _this: &mut Object,
        _cmd: Sel,
        frame_rect: NSRect,
        _screen: id,
    ) -> NSRect {
        frame_rect
    }

    let sel_swizzled_constrainframerect_toscreen = sel!(swizzled_constrainFrameRect:toScreen:);
    unsafe {
        if !HAS_SWIZZLED {
            add_method(
                cls,
                sel_swizzled_constrainframerect_toscreen,
                swizzled_constrainFrameRect_toScreen_
                    as extern "C" fn(&mut Object, Sel, NSRect, id) -> NSRect,
            );
            HAS_SWIZZLED = true;
        } else {
            return;
        }
    };

    unsafe {
        let original = runtime::class_getInstanceMethod(cls, sel!(constrainFrameRect:toScreen:))
            as *mut Method;
        let swizzled =
            runtime::class_getInstanceMethod(cls, sel_swizzled_constrainframerect_toscreen)
                as *mut Method;
        runtime::method_exchangeImplementations(original, swizzled);
    }
}

impl Background {
    pub fn new(event_loop: &EventLoopWindowTarget<ThreadEvent>, app_state: AppState) -> Background {
        let mut inner_size = PhysicalSize::new(1024, 576);
        if let Some(monitor) = event_loop.primary_monitor() {
            inner_size = monitor.size();
        }

        #[cfg(target_os = "macos")]
        swizzle_constrainframerect_toscreen();

        let window: Option<Window>;
        let gfx_context: Option<GfxContext>;

        #[cfg(target_os = "windows")]
        {
            let desktop_hwnd = if let Some(desktop_hwnd) = get_background_hwnd() {
                desktop_hwnd
            } else {
                panic!("failed to get HWND of background window");
            };

            let windows_window = WindowBuilder::new()
                .with_title("shaderbg background")
                .with_decorations(false)
                .with_inner_size(inner_size)
                .with_parent_window(desktop_hwnd)
                .build(event_loop)
                .unwrap();

            gfx_context = Some(GfxContext::new(&windows_window));
            window = Some(windows_window);
        }

        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::{NSScreen, NSWindow, NSWindowCollectionBehavior};
            use tao::platform::macos::WindowExtMacOS;

            let macos_window = WindowBuilder::new()
                .with_title("shaderbg background")
                .with_decorations(false)
                .with_inner_size(inner_size)
                .build(&event_loop)
                .unwrap();

            unsafe {
                let ns_window = window.ns_window() as *mut objc::runtime::Object;

                // place window behind desktop icons
                NSWindow::setLevel_(ns_window, (i32::MIN + 5 + 20 - 1).into());

                // fix window in place, and across all 'desktops'
                NSWindow::setCollectionBehavior_(
                    ns_window,
                    NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle,
                );

                // fit window to screen
                let screen = NSWindow::screen(ns_window) as *mut objc::runtime::Object;
                let rect = NSScreen::frame(screen);
                NSWindow::setFrame_display_(ns_window, rect, true);
            }

            gfx_context = Some(GfxContext::new(&macos_window));
            window = Some(Box::new(macos_window));
        }

        let window = if let Some(window) = window {
            window
        } else {
            panic!("failed to create background window");
        };

        if gfx_context.is_none() {
            panic!("failed to create graphics context");
        };

        Background {
            window,
            app_state,
            gfx_context,
        }
    }

    pub fn run(mut self, rx: mpsc::Receiver<BackgroundEvent>) {
        let size = self.window.inner_size();
        let mut gfx = pollster::block_on(Gfx::new(
            self.gfx_context.take().unwrap(),
            size.width,
            size.height,
            false,
        ));
        let mut shadertoy = ShaderToy::new();

        let mut resources = if let Some(scene) = self.app_state.get().scene() {
            Some(
                Resources::new(
                    scene,
                    &gfx.device,
                    gfx.config.width,
                    gfx.config.height,
                    gfx.config.format,
                )
                .unwrap(),
            )
        } else {
            None
        };

        loop {
            if let Ok(event) = rx.recv() {
                match event {
                    BackgroundEvent::TaoEvent(event) => match event {
                        Event::WindowEvent {
                            event: event::WindowEvent::CloseRequested,
                            ..
                        } => {
                            self.app_state
                                .send(AppEvent::BackgroundClosed(false))
                                .unwrap();
                            break;
                        }
                        Event::MainEventsCleared => self.window.request_redraw(),
                        Event::RedrawEventsCleared => {
                            let time = { *self.app_state.get_time() };
                            let size = self.window.inner_size();
                            shadertoy.update(time.time, time.dt as f64, size.width, size.height);
                            gfx.render(resources.as_mut(), time, shadertoy, None, |_, _| {});
                        }
                        Event::WindowEvent {
                            event: WindowEvent::Resized(PhysicalSize { width, height }),
                            ..
                        } => {
                            gfx.resized(width, height);
                            if let Some(resources) = resources.as_mut() {
                                resources.resize(width, height);
                            }
                        }
                        Event::WindowEvent {
                            event:
                                WindowEvent::ScaleFactorChanged {
                                    new_inner_size: PhysicalSize { width, height },
                                    ..
                                },
                            ..
                        } => {
                            gfx.resized(*width, *height);
                            if let Some(resources) = resources.as_mut() {
                                resources.resize(*width, *height);
                            }
                        }
                        _ => {}
                    },
                    BackgroundEvent::SettingUpdated(key, value) => {
                        if let Some(resources) = resources.as_mut() {
                            resources.update_setting(key, value);
                        }
                    }
                    BackgroundEvent::SceneChanged => {
                        resources = if let Some(scene) = self.app_state.get().scene() {
                            Some(
                                Resources::new(
                                    scene,
                                    &gfx.device,
                                    gfx.config.width,
                                    gfx.config.height,
                                    gfx.config.format,
                                )
                                .unwrap(),
                            )
                        } else {
                            None
                        };
                    }
                    BackgroundEvent::Stop => {
                        self.app_state
                            .send(AppEvent::BackgroundClosed(true))
                            .unwrap();
                        break;
                    }
                }
            }
        }
    }
}

impl Drop for Background {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                SystemParametersInfoA, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER,
            };

            unsafe {
                SystemParametersInfoA(SPI_SETDESKWALLPAPER, 0, None, SPIF_UPDATEINIFILE);
            }
        }
    }
}
