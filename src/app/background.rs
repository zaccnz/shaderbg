use core::fmt::Debug;
use std::sync::mpsc;
use tao::{
    dpi::PhysicalSize,
    event::{self, Event},
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder},
};

use crate::{
    app::{AppState, WindowEvent},
    gfx::{Gfx, GfxContext},
    scene::Resources,
};

use super::AppEvent;

#[derive(Debug)]
pub enum BackgroundEvent {
    TaoEvent(Event<'static, WindowEvent>),
}

pub struct Background {
    pub window: Window,
    app_state: AppState,
    gfx_context: GfxContext,
}

impl Debug for Background {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Background").finish_non_exhaustive()
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
        add_method(
            cls,
            sel_swizzled_constrainframerect_toscreen,
            swizzled_constrainFrameRect_toScreen_
                as extern "C" fn(&mut Object, Sel, NSRect, id) -> NSRect,
        )
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
    pub fn new(event_loop: &EventLoopWindowTarget<WindowEvent>, app_state: AppState) -> Background {
        let mut inner_size = PhysicalSize::new(1024, 576);
        if let Some(monitor) = event_loop.primary_monitor() {
            inner_size = monitor.size();
        }

        #[cfg(target_os = "macos")]
        swizzle_constrainframerect_toscreen();

        let window = WindowBuilder::new()
            .with_title("shaderbg background")
            .with_decorations(false)
            .with_inner_size(inner_size)
            .build(&event_loop)
            .unwrap();

        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::{NSScreen, NSWindow, NSWindowCollectionBehavior};
            use tao::platform::macos::WindowExtMacOS;

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
        }

        let gfx_context = GfxContext::new(&window);

        Background {
            window,
            app_state,
            gfx_context,
        }
    }

    pub fn run(self, rx: mpsc::Receiver<BackgroundEvent>) {
        let mut gfx = Gfx::new(self.gfx_context, &self.window);
        let mut resources = Resources::new(
            self.app_state.clone(),
            &self.app_state.get().scene,
            &gfx.device,
            &gfx.config,
        )
        .unwrap();

        loop {
            match rx.recv() {
                Ok(event) => {
                    //println!("background event: {:?}", event);
                    match event {
                        BackgroundEvent::TaoEvent(event) => match event {
                            Event::WindowEvent {
                                event: event::WindowEvent::CloseRequested,
                                ..
                            } => {
                                self.app_state.send(AppEvent::BackgroundClosed).unwrap();
                                break;
                            }
                            Event::MainEventsCleared => self.window.request_redraw(),
                            Event::RedrawEventsCleared => {
                                gfx.render(&self.window, Some(&mut resources), None);
                            }
                            _ => {}
                        },
                    }
                }
                _ => {}
            }
        }
    }
}
