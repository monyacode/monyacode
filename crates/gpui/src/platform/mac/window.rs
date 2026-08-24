use super::MacDisplay;
use crate::{
    AnyWindowHandle, Bounds, Capslock, DevicePixels, DisplayLink, ExternalPaths, FileDropEvent, ForegroundExecutor,
    KeyDownEvent, Keystroke, Modifiers, MouseMoveEvent, Pixels, PlatformAtlas, PlatformDisplay, PlatformInput,
    PlatformWindow, Point, PromptButton, PromptLevel, RequestFrameOptions, SharedString, Size, SystemWindowTab, Timer,
    WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowKind, WindowParams,
    platform::{
        PlatformInputHandler,
        mac::{blurred_view::BlurredView, events::ESCAPE_KEY, gpui_view::GPUIView},
        wgpu::{WgpuContext, WgpuRenderer, WgpuSurfaceConfig},
    },
    point, px, screen_display_id, size,
};
use block2::RcBlock;
use futures::channel::oneshot;
use objc2::{
    AnyThread, ClassType, DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSAppearanceCustomization, NSApplication, NSAutoresizingMaskOptions, NSBackingStoreType,
    NSButton, NSColor, NSDragOperation, NSDraggingInfo, NSEvent, NSEventModifierFlags, NSNormalWindowLevel, NSPanel,
    NSPasteboardTypeFileURL, NSPopUpMenuWindowLevel, NSScreen, NSTextInputContext, NSTitlebarAccessoryViewController,
    NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindow, NSWindowAnimationBehavior, NSWindowButton,
    NSWindowCollectionBehavior, NSWindowDelegate, NSWindowOcclusionState, NSWindowOrderingMode, NSWindowStyleMask,
    NSWindowTabbingMode, NSWindowTitleVisibility,
};
use objc2_core_foundation::CGPoint;
use objc2_foundation::{
    MainThreadMarker, NSArray, NSCopying, NSDictionary, NSMutableIndexSet, NSNotification,
    NSObjectNSScriptClassDescription, NSObjectProtocol, NSOperatingSystemVersion, NSPoint, NSProcessInfo, NSRange,
    NSRect, NSSize, NSString, NSURL, NSUserDefaults, ns_string,
};
use objc2_quartz_core::CALayer;
use parking_lot::Mutex;
use raw_window_handle as rwh;
use smallvec::SmallVec;
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    path::PathBuf,
    ptr::NonNull,
    rc::Rc,
    sync::{Arc, Weak},
    time::Duration,
};
use util::ResultExt;
use wgpu::PresentMode;

#[derive(PartialEq)]
pub enum UserTabbingPreference {
    Never,
    Always,
    InFullScreen,
}

pub struct GpuiWindowIvars {
    window_state: RefCell<Option<Arc<Mutex<MacWindowState>>>>,
}

pub trait GpuiWindowShared {
    fn get_ivars(&self) -> &GpuiWindowIvars;

    fn state(&self) -> Arc<Mutex<MacWindowState>> {
        self.get_ivars()
            .window_state
            .borrow()
            .as_ref()
            .expect("valid window_state")
            .clone()
    }

    fn window_did_change_occlusion_state(&self, _: &AnyObject) {
        let window_state = self.state();
        let lock = &mut *window_state.lock();
        if lock
            .native_window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible)
        {
            lock.move_traffic_light();
            lock.start_display_link();
        } else {
            lock.stop_display_link();
        }
    }

    fn window_did_resize(&self, _: &AnyObject) {
        let window_state = self.state();
        window_state.as_ref().lock().move_traffic_light();
    }

    fn window_will_enter_fullscreen(&self, _: &AnyObject) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        lock.fullscreen_restore_bounds = lock.bounds();

        if is_macos_version_at_least(15, 3, 0) {
            lock.native_window.setTitlebarAppearsTransparent(false);
        }
    }

    fn window_will_exit_fullscreen(&self, _: &AnyObject) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();

        if is_macos_version_at_least(15, 3, 0) && lock.transparent_titlebar {
            lock.native_window.setTitlebarAppearsTransparent(false);
        }
    }

    fn window_did_move(&self, _: &AnyObject) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.moved_callback.take() {
            drop(lock);
            callback();
            window_state.lock().moved_callback = Some(callback);
        }
    }

    fn window_did_change_screen(&self, _: &AnyObject) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        lock.start_display_link();
        drop(lock);
        update_window_scale_factor(&window_state);
    }

    fn window_did_change_key_status(&self, cmd: Sel, _: &AnyObject) {
        let window_state = self.state();
        let mut lock = window_state.lock();
        let is_active = lock.native_window.isKeyWindow();

        // When opening a pop-up while the application isn't active, Cocoa sends a spurious
        // `windowDidBecomeKey` message to the previous key window even though that window
        // isn't actually key. This causes a bug if the application is later activated while
        // the pop-up is still open, making it impossible to activate the previous key window
        // even if the pop-up gets closed. The only way to activate it again is to de-activate
        // the app and re-activate it, which is a pretty bad UX.
        // The following code detects the spurious event and invokes `resignKeyWindow`:
        // in theory, we're not supposed to invoke this method manually but it balances out
        // the spurious `becomeKeyWindow` event and helps us work around that bug.
        if cmd == sel!(windowDidBecomeKey:) && !is_active {
            lock.native_window.resignKeyWindow();
            return;
        }

        let executor = lock.executor.clone();
        drop(lock);

        // When a window becomes active, trigger an immediate synchronous frame request to prevent
        // tab flicker when switching between windows in native tabs mode.
        //
        // This is only done on subsequent activations (not the first) to ensure the initial focus
        // path is properly established. Without this guard, the focus state would remain unset until
        // the first mouse click, causing keybindings to be non-functional.
        if cmd == sel!(windowDidBecomeKey:) && is_active {
            let window_state = self.state();
            let mut lock = window_state.lock();

            if lock.activated_least_once {
                if let Some(mut callback) = lock.request_frame_callback.take() {
                    // lock.renderer.set_presents_with_transaction(true);
                    lock.stop_display_link();
                    drop(lock);
                    callback(Default::default());

                    let mut lock = window_state.lock();
                    lock.request_frame_callback = Some(callback);
                    // lock.renderer.set_presents_with_transaction(false);
                    lock.start_display_link();
                }
            } else {
                lock.activated_least_once = true;
            }
        }

        executor
            .spawn(async move {
                let mut lock = window_state.as_ref().lock();
                if is_active {
                    lock.move_traffic_light();
                }

                if let Some(mut callback) = lock.activate_callback.take() {
                    drop(lock);
                    callback(is_active);
                    window_state.lock().activate_callback = Some(callback);
                };
            })
            .detach();
    }

    fn window_should_close(&self, _: &AnyObject) -> bool {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.should_close_callback.take() {
            drop(lock);
            let should_close = callback();
            window_state.lock().should_close_callback = Some(callback);
            should_close
        } else {
            true
        }
    }

    fn close_window_callback(&self) {
        let close_callback = {
            let window_state = self.state();
            let mut lock = window_state.as_ref().lock();
            lock.close_callback.take()
        };

        if let Some(callback) = close_callback {
            callback();
        }
    }

    fn dragging_entered(&self, dragging_info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
        let window_state = self.state();
        let position = drag_event_position(&window_state, dragging_info);
        let paths = external_paths_from_event(dragging_info);
        if let Some(event) = paths.map(|paths| PlatformInput::FileDrop(FileDropEvent::Entered { position, paths }))
            && send_new_event(&window_state, event)
        {
            window_state.lock().external_files_dragged = true;
            return NSDragOperation::Copy;
        }
        NSDragOperation::None
    }

    fn dragging_updated(&self, dragging_info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
        let window_state = self.state();
        let position = drag_event_position(&window_state, dragging_info);
        if send_new_event(
            &window_state,
            PlatformInput::FileDrop(FileDropEvent::Pending { position }),
        ) {
            NSDragOperation::Copy
        } else {
            NSDragOperation::None
        }
    }

    fn dragging_exited(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
        let window_state = self.state();
        send_new_event(&window_state, PlatformInput::FileDrop(FileDropEvent::Exited));
        window_state.lock().external_files_dragged = false;
    }

    fn perform_drag_operation(&self, dragging_info: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
        let window_state = self.state();
        let position = drag_event_position(&window_state, dragging_info);
        send_new_event(
            &window_state,
            PlatformInput::FileDrop(FileDropEvent::Submit { position }),
        )
    }

    fn conclude_drag_operation(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
        let window_state = self.state();
        send_new_event(&window_state, PlatformInput::FileDrop(FileDropEvent::Exited));
    }

    fn add_titlebar_accessory_view_controller(&self, view_controller: &NSTitlebarAccessoryViewController) {
        // Hide the native tab bar and set its height to 0, since we render our own.
        let accessory_view = view_controller.view();
        accessory_view.setHidden(true);
        let mut frame = accessory_view.frame();
        frame.size.height = 0.0;
        accessory_view.setFrame(frame);
    }

    fn move_tab_to_new_window(&self, _: Option<&AnyObject>) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.move_tab_to_new_window_callback.take() {
            drop(lock);
            callback();
            window_state.lock().move_tab_to_new_window_callback = Some(callback);
        }
    }

    fn merge_all_windows(&self, _: Option<&AnyObject>) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.merge_all_windows_callback.take() {
            drop(lock);
            callback();
            window_state.lock().merge_all_windows_callback = Some(callback);
        }
    }

    fn select_next_tab(&self, _: Option<&AnyObject>) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.select_next_tab_callback.take() {
            drop(lock);
            callback();
            window_state.lock().select_next_tab_callback = Some(callback);
        }
    }

    fn select_previous_tab(&self, _: Option<&AnyObject>) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.select_previous_tab_callback.take() {
            drop(lock);
            callback();
            window_state.lock().select_previous_tab_callback = Some(callback);
        }
    }

    fn toggle_tab_bar(&self, _: Option<&AnyObject>) {
        let window_state = self.state();
        let mut lock = window_state.as_ref().lock();
        lock.move_traffic_light();

        if let Some(mut callback) = lock.toggle_tab_bar_callback.take() {
            drop(lock);
            callback();
            window_state.lock().toggle_tab_bar_callback = Some(callback);
        }
    }
}

define_class!(
    #[unsafe(super(NSWindow))]
    #[name = "GpuiWindow"]
    #[ivars = GpuiWindowIvars]
    #[thread_kind = MainThreadOnly]
    pub struct GpuiWindow;

    unsafe impl NSObjectProtocol for GpuiWindow {}

    unsafe impl NSWindowDelegate for GpuiWindow {
        #[unsafe(method(windowDidResize:))]
        fn _window_did_resize(&self, notification: &NSNotification) {
            self.window_did_resize(notification)
        }


        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn _window_did_change_occlusion_state(&self, notification: &NSNotification) {
            self.window_did_change_occlusion_state(notification)
        }

        #[unsafe(method(windowWillEnterFullScreen:))]
        fn _window_will_enter_fullscreen(&self, notification: &NSNotification) {
            self.window_will_enter_fullscreen(notification)
        }

        #[unsafe(method(windowWillExitFullScreen:))]
        fn _window_will_exit_fullscreen(&self, notification: &NSNotification) {
            self.window_will_exit_fullscreen(notification)
        }

        #[unsafe(method(windowDidMove:))]
        fn _window_did_move(&self, notification: &NSNotification) {
            self.window_did_move(notification)
        }

        #[unsafe(method(windowDidChangeScreen:))]
        fn _window_did_change_screen(&self, notification: &NSNotification) {
            self.window_did_change_screen(notification)
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn _window_did_become_key(&self, notification: &NSNotification) {
            self.window_did_change_key_status(sel!(windowDidBecomeKey:), notification)
        }

        #[unsafe(method(windowDidResignKey:))]
        fn _window_did_resign_key(&self, notification: &NSNotification) {
            self.window_did_change_key_status(sel!(windowDidResignKey:), notification)
        }

        #[unsafe(method(windowShouldClose:))]
        fn _window_should_close(&self, notification: &NSNotification) -> bool {
            self.window_should_close(notification)
        }
    }

    impl GpuiWindow {
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

    #[unsafe(method(close))]
    fn _close_window(&self) {
        self.close_window_callback();
        unsafe { msg_send![super(self), close] }
    }

    #[unsafe(method(draggingEntered:))]
    fn _dragging_entered(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NSDragOperation {
        self.dragging_entered(dragging_info)
    }

    #[unsafe(method(draggingUpdated:))]
    fn _dragging_updated(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NSDragOperation {
        self.dragging_updated(dragging_info)
    }

    #[unsafe(method(draggingExited:))]
    fn _dragging_exited(&self, dragging_info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
        self.dragging_exited(dragging_info)
    }

    #[unsafe(method(performDragOperation:))]
    fn _perform_drag_operation(&self, dragging_info: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
        self.perform_drag_operation(dragging_info)
    }

    #[unsafe(method(concludeDragOperation:))]
    fn _conclude_drag_operation(&self, dragging_info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
        self.conclude_drag_operation(dragging_info)
    }

    #[unsafe(method(addTitlebarAccessoryViewController:))]
    fn _add_titlebar_accessory_view_controller(
        &self,
        view_controller: &NSTitlebarAccessoryViewController,
    ) {
        let _:() = unsafe { msg_send![super(self), addTitlebarAccessoryViewController: view_controller] };
        self.add_titlebar_accessory_view_controller(view_controller)
    }

    #[unsafe(method(moveTabToNewWindow:))]
    fn _move_tab_to_new_window(&self, sender: Option<&AnyObject>) {
        let _:() = unsafe { msg_send![super(self), moveTabToNewWindow: sender] };
        self.move_tab_to_new_window(sender)
    }

    #[unsafe(method(mergeAllWindows:))]
    fn _merge_all_windows(&self, sender: Option<&AnyObject>) {
        let _:() = unsafe { msg_send![super(self), mergeAllWindows: sender] };
        self.merge_all_windows(sender)
    }

    #[unsafe(method(selectNextTab:))]
    fn _select_next_tab(&self, sender: Option<&AnyObject>) {
        self.select_next_tab(sender)
    }

    #[unsafe(method(selectPreviousTab:))]
    fn _select_previous_tab(&self, sender: Option<&AnyObject>) {
        self.select_previous_tab(sender)
    }

    #[unsafe(method(toggleTabBar:))]
    fn _toggle_tab_bar(&self, sender: Option<&AnyObject>) {
        let _:() = unsafe { msg_send![super(self), toggleTabBar: sender] };
        self.toggle_tab_bar(sender)
    }
    }
);

impl GpuiWindow {
    fn new(
        mtm: MainThreadMarker,
        rect: NSRect,
        style_mask: NSWindowStyleMask,
        screen: Option<&NSScreen>,
    ) -> Retained<Self> {
        let this = GpuiWindow::alloc(mtm);
        let this = this.set_ivars(GpuiWindowIvars {
            window_state: RefCell::new(None),
        });
        unsafe {
            msg_send![
                super(this),
                initWithContentRect: rect,
                styleMask: style_mask,
                backing: NSBackingStoreType::Buffered,
                defer: false,
                screen: screen,
            ]
        }
    }

    fn set_window_state(&self, state: Arc<Mutex<MacWindowState>>) {
        *self.ivars().window_state.borrow_mut() = Some(state);
    }
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "GpuiPanel"]
    #[ivars = GpuiWindowIvars]
    #[thread_kind = MainThreadOnly]
    pub struct GpuiPanel;

    unsafe impl NSObjectProtocol for GpuiPanel {}

    unsafe impl NSWindowDelegate for GpuiPanel {}

    impl GpuiPanel {
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }
    }
);

impl GpuiPanel {
    fn new(
        mtm: MainThreadMarker,
        rect: NSRect,
        style_mask: NSWindowStyleMask,
        screen: Option<&NSScreen>,
    ) -> Retained<Self> {
        let this = GpuiPanel::alloc(mtm);
        let this = this.set_ivars(GpuiWindowIvars {
            window_state: RefCell::new(None),
        });
        unsafe {
            msg_send![
                super(this),
                initWithContentRect: rect,
                styleMask: style_mask | NSWindowStyleMask::NonactivatingPanel,
                backing: NSBackingStoreType::Buffered,
                defer: false,
                screen: screen,
            ]
        }
    }

    fn set_window_state(&self, state: Arc<Mutex<MacWindowState>>) {
        *self.ivars().window_state.borrow_mut() = Some(state);
    }
}

impl GpuiWindowShared for GpuiWindow {
    fn get_ivars(&self) -> &GpuiWindowIvars {
        self.ivars()
    }
}

impl GpuiWindowShared for GpuiPanel {
    fn get_ivars(&self) -> &GpuiWindowIvars {
        self.ivars()
    }
}

pub(crate) struct MacWindowState {
    handle: AnyWindowHandle,
    pub(crate) executor: ForegroundExecutor,
    pub(crate) native_window: Retained<NSWindow>,
    native_view: Retained<GPUIView>,
    blurred_view: Option<Retained<BlurredView>>,
    background_appearance: WindowBackgroundAppearance,
    display_link: Option<DisplayLink>,
    pub(crate) renderer: WgpuRenderer,
    pub(crate) request_frame_callback: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(crate) event_callback: Option<Box<dyn FnMut(PlatformInput) -> crate::DispatchEventResult>>,
    activate_callback: Option<Box<dyn FnMut(bool)>>,
    pub(crate) resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    moved_callback: Option<Box<dyn FnMut()>>,
    should_close_callback: Option<Box<dyn FnMut() -> bool>>,
    close_callback: Option<Box<dyn FnOnce()>>,
    pub(crate) appearance_changed_callback: Option<Box<dyn FnMut()>>,
    pub(crate) input_handler: Option<PlatformInputHandler>,
    pub(crate) last_key_equivalent: Option<KeyDownEvent>,
    pub(crate) synthetic_drag_counter: usize,
    traffic_light_position: Option<Point<Pixels>>,
    transparent_titlebar: bool,
    pub(crate) previous_modifiers_changed_event: Option<PlatformInput>,
    pub(crate) keystroke_for_do_command: Option<Keystroke>,
    pub(crate) do_command_handled: Option<bool>,
    pub(crate) external_files_dragged: bool,
    // Whether the next left-mouse click is also the focusing click.
    pub(crate) first_mouse: bool,
    fullscreen_restore_bounds: Bounds<Pixels>,
    move_tab_to_new_window_callback: Option<Box<dyn FnMut()>>,
    merge_all_windows_callback: Option<Box<dyn FnMut()>>,
    select_next_tab_callback: Option<Box<dyn FnMut()>>,
    select_previous_tab_callback: Option<Box<dyn FnMut()>>,
    toggle_tab_bar_callback: Option<Box<dyn FnMut()>>,
    activated_least_once: bool,
}

impl MacWindowState {
    pub(crate) fn move_traffic_light(&self) {
        let Some(traffic_light_position) = self.traffic_light_position else {
            return;
        };
        if self.is_fullscreen() {
            // Moving traffic lights while fullscreen doesn't work,
            // see https://github.com/zed-industries/zed/issues/4712
            return;
        }

        let titlebar_height = self.titlebar_height();

        let close_button = self
            .native_window
            .standardWindowButton(NSWindowButton::CloseButton)
            .unwrap();
        let min_button = self
            .native_window
            .standardWindowButton(NSWindowButton::MiniaturizeButton)
            .unwrap();
        let zoom_button = self
            .native_window
            .standardWindowButton(NSWindowButton::ZoomButton)
            .unwrap();
        let mut close_button_frame = close_button.frame();
        let mut min_button_frame = min_button.frame();
        let mut zoom_button_frame = zoom_button.frame();
        let mut origin = point(
            traffic_light_position.x,
            titlebar_height - traffic_light_position.y - px(close_button_frame.size.height as f32),
        );
        let button_spacing = px((min_button_frame.origin.x - close_button_frame.origin.x) as f32);

        close_button_frame.origin = CGPoint::new(origin.x.into(), origin.y.into());
        close_button.setFrame(close_button_frame);
        origin.x += button_spacing;

        min_button_frame.origin = CGPoint::new(origin.x.into(), origin.y.into());
        min_button.setFrame(min_button_frame);
        origin.x += button_spacing;

        zoom_button_frame.origin = CGPoint::new(origin.x.into(), origin.y.into());
        zoom_button.setFrame(zoom_button_frame);
        origin.x += button_spacing;
    }

    pub fn start_display_link(&mut self) {
        self.stop_display_link();
        if !self
            .native_window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible)
        {
            return;
        }
        let Some(screen) = self.native_window.screen() else {
            return;
        };
        let display_id = screen_display_id(&screen);
        if let Some(mut display_link) =
            DisplayLink::new(display_id, Retained::as_ptr(&self.native_view) as *mut c_void, step).log_err()
        {
            display_link.start().log_err();
            self.display_link = Some(display_link);
        }
    }

    pub fn stop_display_link(&mut self) {
        self.display_link = None;
    }

    fn is_maximized(&self) -> bool {
        let Some(screen) = self.native_window.screen() else {
            return false;
        };
        let bounds = self.bounds();
        let screen_size = screen.visibleFrame().into();
        bounds.size == screen_size
    }

    fn is_fullscreen(&self) -> bool {
        let style_mask = self.native_window.styleMask();
        style_mask.contains(NSWindowStyleMask::FullScreen)
    }

    fn bounds(&self) -> Bounds<Pixels> {
        let mut window_frame = self.native_window.frame();
        let Some(screen) = self.native_window.screen() else {
            return Bounds::new(point(px(0.), px(0.)), crate::DEFAULT_WINDOW_SIZE);
        };
        let screen_frame = screen.frame();

        // Flip the y coordinate to be top-left origin
        window_frame.origin.y = screen_frame.size.height - window_frame.origin.y - window_frame.size.height;

        Bounds::new(
            point(
                px((window_frame.origin.x - screen_frame.origin.x) as f32),
                px((window_frame.origin.y + screen_frame.origin.y) as f32),
            ),
            size(px(window_frame.size.width as f32), px(window_frame.size.height as f32)),
        )
    }

    pub fn content_size(&self) -> Size<Pixels> {
        let Some(content_view) = self.native_window.contentView() else {
            return size(px(0.0), px(0.0));
        };
        let frame = content_view.frame();
        size(px(frame.size.width as f32), px(frame.size.height as f32))
    }

    pub fn scale_factor(&self) -> f32 {
        get_scale_factor(&self.native_window)
    }

    fn titlebar_height(&self) -> Pixels {
        let frame = self.native_window.frame();
        let content_layout_rect = self.native_window.contentLayoutRect();
        px((frame.size.height - content_layout_rect.size.height) as f32)
    }

    fn window_bounds(&self) -> WindowBounds {
        if self.is_fullscreen() {
            WindowBounds::Fullscreen(self.fullscreen_restore_bounds)
        } else {
            WindowBounds::Windowed(self.bounds())
        }
    }
}

unsafe impl Send for MacWindowState {}

pub(crate) struct MacWindow(Arc<Mutex<MacWindowState>>);

struct RawWindow {
    view: Retained<GPUIView>,
}

unsafe impl Send for RawWindow {}
unsafe impl Sync for RawWindow {}

impl rwh::HasWindowHandle for RawWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let view = NonNull::<c_void>::new(Retained::as_ptr(&self.view) as *mut c_void).unwrap();
        let handle = rwh::AppKitWindowHandle::new(view);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl rwh::HasDisplayHandle for RawWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let handle = rwh::RawDisplayHandle::AppKit(rwh::AppKitDisplayHandle::new());
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle) })
    }
}

enum GpuiWindowVariant {
    Window(Retained<GpuiWindow>),
    Panel(Retained<GpuiPanel>),
}

impl MacWindow {
    pub fn open(
        handle: AnyWindowHandle,
        WindowParams {
            bounds,
            titlebar,
            kind,
            is_movable,
            is_resizable,
            is_minimizable,
            focus,
            show,
            display_id,
            window_min_size,
            tabbing_identifier,
        }: WindowParams,
        executor: ForegroundExecutor,
        renderer_context: &WgpuContext,
    ) -> Self {
        let mtm = MainThreadMarker::new().unwrap();
        let allows_automatic_window_tabbing = tabbing_identifier.is_some();
        NSWindow::setAllowsAutomaticWindowTabbing(allows_automatic_window_tabbing, mtm);

        let mut style_mask;
        if let Some(titlebar) = titlebar.as_ref() {
            style_mask = NSWindowStyleMask::Closable | NSWindowStyleMask::Titled;

            if is_resizable {
                style_mask |= NSWindowStyleMask::Resizable;
            }

            if is_minimizable {
                style_mask |= NSWindowStyleMask::Miniaturizable;
            }

            if titlebar.appears_transparent {
                style_mask |= NSWindowStyleMask::FullSizeContentView;
            }
        } else {
            style_mask = NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView;
        }

        let display = display_id
            .and_then(MacDisplay::find_by_id)
            .unwrap_or_else(MacDisplay::primary);

        let mut target_screen: Option<Retained<NSScreen>> = None;
        let mut screen_frame = None;

        let screens = NSScreen::screens(mtm);
        let count = screens.count();
        for i in 0..count {
            let screen = screens.objectAtIndex(i);
            let frame = screen.frame();
            let display_id = screen_display_id(&screen);
            if display_id == display.0 {
                screen_frame = Some(frame);
                target_screen = Some(screen);
            }
        }

        let screen_frame = screen_frame.unwrap_or_else(|| {
            let screen = NSScreen::mainScreen(mtm);
            let frame = screen.as_ref().map(|s| s.frame());
            target_screen = screen;
            frame.unwrap()
        });

        let window_rect = NSRect::new(
            NSPoint::new(
                screen_frame.origin.x + bounds.origin.x.0 as f64,
                screen_frame.origin.y + (display.bounds().size.height - bounds.origin.y).0 as f64,
            ),
            NSSize::new(bounds.size.width.0 as f64, bounds.size.height.0 as f64),
        );

        let native_window = match kind {
            WindowKind::Normal | WindowKind::Floating => {
                GpuiWindowVariant::Window(GpuiWindow::new(mtm, window_rect, style_mask, target_screen.as_deref()))
            }
            WindowKind::PopUp => {
                GpuiWindowVariant::Panel(GpuiPanel::new(mtm, window_rect, style_mask, target_screen.as_deref()))
            }
        };

        let nswindow = match &native_window {
            GpuiWindowVariant::Window(retained) => retained.clone().into_super(),
            GpuiWindowVariant::Panel(retained) => retained.clone().into_super().into_super(),
        };

        unsafe {
            nswindow.registerForDraggedTypes(&NSArray::from_retained_slice(&[NSPasteboardTypeFileURL.copy()]));
            nswindow.setReleasedWhenClosed(false);
        }

        let content_view = nswindow.contentView();
        let native_view = {
            let mtm = MainThreadMarker::new().expect("Must be called from the main thread");
            let bounds = content_view.as_ref().unwrap().bounds();
            crate::platform::mac::gpui_view::GPUIView::new(mtm, bounds)
        };

        let renderer = {
            let raw_window = RawWindow {
                view: native_view.clone(),
            };
            let surface_config = WgpuSurfaceConfig {
                size: Size {
                    width: DevicePixels(bounds.size.width.0 as i32),
                    height: DevicePixels(bounds.size.height.0 as i32),
                },
                transparent: true,
                preferred_present_mode: Some(PresentMode::Fifo),
            };

            WgpuRenderer::new(renderer_context, &raw_window, surface_config).unwrap()
        };

        let mut window = Self(Arc::new(Mutex::new(MacWindowState {
            handle,
            executor,
            native_window: nswindow.clone(),
            native_view: native_view.clone(),
            blurred_view: None,
            background_appearance: WindowBackgroundAppearance::Opaque,
            display_link: None,
            renderer,
            request_frame_callback: None,
            event_callback: None,
            activate_callback: None,
            resize_callback: None,
            moved_callback: None,
            should_close_callback: None,
            close_callback: None,
            appearance_changed_callback: None,
            input_handler: None,
            last_key_equivalent: None,
            synthetic_drag_counter: 0,
            traffic_light_position: titlebar.as_ref().and_then(|titlebar| titlebar.traffic_light_position),
            transparent_titlebar: titlebar.as_ref().is_none_or(|titlebar| titlebar.appears_transparent),
            previous_modifiers_changed_event: None,
            keystroke_for_do_command: None,
            do_command_handled: None,
            external_files_dragged: false,
            first_mouse: false,
            fullscreen_restore_bounds: Bounds::default(),
            move_tab_to_new_window_callback: None,
            merge_all_windows_callback: None,
            select_next_tab_callback: None,
            select_previous_tab_callback: None,
            toggle_tab_bar_callback: None,
            activated_least_once: false,
        })));

        match &native_window {
            GpuiWindowVariant::Window(wnd) => {
                wnd.set_window_state(window.0.clone());
                wnd.setDelegate(Some(&ProtocolObject::from_ref(&**wnd)));
            }
            GpuiWindowVariant::Panel(wnd) => {
                wnd.set_window_state(window.0.clone());
                wnd.setDelegate(Some(&ProtocolObject::from_ref(&**wnd)));
            }
        }
        native_view.set_window_state(window.0.clone());

        if let Some(title) = titlebar.as_ref().and_then(|t| t.title.as_ref().map(AsRef::as_ref)) {
            window.set_title(title);
        }

        nswindow.setMovable(is_movable);

        if let Some(window_min_size) = window_min_size {
            nswindow.setContentMinSize(NSSize {
                width: window_min_size.width.to_f64(),
                height: window_min_size.height.to_f64(),
            });
        }

        if titlebar.is_none_or(|titlebar| titlebar.appears_transparent) {
            nswindow.setTitlebarAppearsTransparent(true);
            nswindow.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        }

        native_view.setAutoresizingMask(
            objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        #[allow(deprecated)]
        native_view.setWantsBestResolutionOpenGLSurface(true);

        // From winit crate: On Mojave, views automatically become layer-backed shortly after
        // being added to a native_window. Changing the layer-backedness of a view breaks the
        // association between the view and its associated OpenGL context. To work around this,
        // on we explicitly make the view layer-backed up front so that AppKit doesn't do it
        // itself and break the association with its context.
        native_view.setWantsLayer(true);
        native_view.setLayerContentsRedrawPolicy(objc2_app_kit::NSViewLayerContentsRedrawPolicy::DuringViewResize);

        content_view.inspect(|view| {
            view.addSubview(&native_view);
        });
        nswindow.makeFirstResponder(Some(&*native_view));

        match native_window {
            GpuiWindowVariant::Window(window) => {
                window.setLevel(NSNormalWindowLevel);
                window.setAcceptsMouseMovedEvents(true);

                if let Some(tabbing_identifier) = tabbing_identifier {
                    let tabbing_id = NSString::from_str(tabbing_identifier.as_str());
                    window.setTabbingIdentifier(&tabbing_id);
                } else {
                    window.setTabbingMode(NSWindowTabbingMode::Disallowed);
                }
            }
            GpuiWindowVariant::Panel(panel) => {
                // Use a tracking area to allow receiving MouseMoved events even when
                // the window or application aren't active, which is often the case
                // e.g. for notification windows.
                let options = NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::MouseMoved
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect;
                let tracking_area = unsafe {
                    NSTrackingArea::initWithRect_options_owner_userInfo(
                        NSTrackingArea::alloc(),
                        NSRect::ZERO,
                        options,
                        Some(&*native_view),
                        None,
                    )
                };
                native_view.addTrackingArea(&tracking_area);

                panel.setLevel(NSPopUpMenuWindowLevel);
                panel.setAnimationBehavior(NSWindowAnimationBehavior::UtilityWindow);
                panel.setCollectionBehavior(
                    NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary,
                );
            }
        }

        let app = NSApplication::sharedApplication(mtm);
        let main_window = app.mainWindow();
        if allows_automatic_window_tabbing && !main_window.is_none() && main_window.as_ref() != Some(&nswindow) {
            let main_window_is_fullscreen = nswindow.styleMask().contains(NSWindowStyleMask::FullScreen);
            let user_tabbing_preference =
                Self::get_user_tabbing_preference().unwrap_or(UserTabbingPreference::InFullScreen);
            let should_add_as_tab = user_tabbing_preference == UserTabbingPreference::Always
                || user_tabbing_preference == UserTabbingPreference::InFullScreen && main_window_is_fullscreen;

            if should_add_as_tab {
                let main_window_can_tab = main_window
                    .as_ref()
                    .map(|w| w.respondsToSelector(sel!(addTabbedWindow:ordered:)))
                    == Some(true);
                let main_window_visible = main_window.as_ref().map(|w| w.isVisible()) == Some(true);

                if main_window_can_tab
                    && main_window_visible
                    && let Some(main_window) = main_window.as_ref()
                {
                    main_window.addTabbedWindow_ordered(&nswindow, NSWindowOrderingMode::Above);

                    // Ensure the window is visible immediately after adding the tab, since the tab bar is updated with a new entry at this point.
                    // Note: Calling orderFront here can break fullscreen mode (makes fullscreen windows exit fullscreen), so only do this if the main window is not fullscreen.
                    if !main_window_is_fullscreen {
                        nswindow.orderFront(None);
                    }
                }
            }
        }

        if focus && show {
            nswindow.makeKeyAndOrderFront(None);
        } else if show {
            nswindow.orderFront(None);
        }

        // Set the initial position of the window to the specified origin.
        // Although we already specified the position using `initWithContentRect_styleMask_backing_defer_screen_`,
        // the window position might be incorrect if the main screen (the screen that contains the window that has focus)
        //  is different from the primary screen.
        nswindow.setFrameTopLeftPoint(window_rect.origin);
        window.0.lock().move_traffic_light();

        window
    }

    pub fn active_window() -> Option<AnyWindowHandle> {
        let mtm = MainThreadMarker::new()?;
        let app = NSApplication::sharedApplication(mtm);
        let main_window = app.mainWindow()?;

        to_window_state(&main_window).map(|window_state| window_state.lock().handle)
    }

    pub fn ordered_windows() -> Vec<AnyWindowHandle> {
        let mtm = MainThreadMarker::new().expect("Must run on the main thread");
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.orderedWindows();
        let count = windows.count();

        let mut window_handles = Vec::new();
        for i in 0..count {
            let window = windows.objectAtIndex(i);
            let handle = to_window_state(&window).map(|window_state| window_state.lock().handle);
            if let Some(handle) = handle {
                window_handles.push(handle);
            }
        }

        window_handles
    }

    pub fn get_user_tabbing_preference() -> Option<UserTabbingPreference> {
        let defaults = NSUserDefaults::standardUserDefaults();
        let domain = ns_string!("NSGlobalDomain");
        let key = ns_string!("AppleWindowTabbingMode");

        let dict = defaults.persistentDomainForName(domain);
        let value = dict
            .and_then(|dict| dict.objectForKey(key))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|value| value.to_string())
            .unwrap_or_default();

        match value.as_ref() {
            "manual" => Some(UserTabbingPreference::Never),
            "always" => Some(UserTabbingPreference::Always),
            _ => Some(UserTabbingPreference::InFullScreen),
        }
    }
}

fn to_window_state(window: &NSWindow) -> Option<Arc<Mutex<MacWindowState>>> {
    if let Some(window) = window.downcast_ref::<GpuiWindow>() {
        Some(window.state())
    } else if let Some(panel) = window.downcast_ref::<GpuiPanel>() {
        Some(panel.state())
    } else {
        None
    }
}

impl Drop for MacWindow {
    fn drop(&mut self) {
        let mut this = self.0.lock();
        this.renderer.destroy();
        let window = this.native_window.clone();
        this.display_link.take();
        this.native_window.setDelegate(None);
        this.input_handler.take();
        this.executor
            .spawn(async move {
                window.close();
            })
            .detach();
    }
}

impl PlatformWindow for MacWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.as_ref().lock().bounds()
    }

    fn window_bounds(&self) -> WindowBounds {
        self.0.as_ref().lock().window_bounds()
    }

    fn is_maximized(&self) -> bool {
        self.0.as_ref().lock().is_maximized()
    }

    fn content_size(&self) -> Size<Pixels> {
        self.0.as_ref().lock().content_size()
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                window.setContentSize(NSSize {
                    width: size.width.0 as f64,
                    height: size.height.0 as f64,
                });
            })
            .detach();
    }

    fn merge_all_windows(&self) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                window.mergeAllWindows(None);
            })
            .detach();
    }

    fn move_tab_to_new_window(&self) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                window.moveTabToNewWindow(None);
                window.makeKeyAndOrderFront(None);
            })
            .detach();
    }

    fn toggle_window_tab_overview(&self) {
        self.0.lock().native_window.toggleTabOverview(None);
    }

    fn set_tabbing_identifier(&self, tabbing_identifier: Option<String>) {
        let mtm = MainThreadMarker::new().expect("Must run on the main thread");
        let lock = self.0.lock();
        NSWindow::setAllowsAutomaticWindowTabbing(tabbing_identifier.is_some(), mtm);
        lock.native_window.setTabbingIdentifier(
            &tabbing_identifier
                .as_ref()
                .map(|ti| NSString::from_str(ti))
                .unwrap_or_default(),
        );
    }

    fn set_traffic_light_visible(&self, visible: bool) {
        let this = self.0.lock();
        let native_window = this.native_window.clone();
        this.executor
            .spawn(async move {
                let buttons = [
                    NSWindowButton::CloseButton,
                    NSWindowButton::MiniaturizeButton,
                    NSWindowButton::ZoomButton,
                ];
                for button in buttons {
                    if let Some(button) = native_window.standardWindowButton(button) {
                        button.setHidden(!visible);
                    }
                }
            })
            .detach();
    }

    fn scale_factor(&self) -> f32 {
        self.0.as_ref().lock().scale_factor()
    }

    fn appearance(&self) -> WindowAppearance {
        let appearance = self.0.lock().native_window.effectiveAppearance();
        WindowAppearance::from_native(&appearance)
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        let screen = self.0.lock().native_window.screen()?;
        Some(Rc::new(MacDisplay(screen_display_id(&screen))))
    }

    fn mouse_position(&self) -> Point<Pixels> {
        let position = self.0.lock().native_window.mouseLocationOutsideOfEventStream();
        convert_mouse_position(position, self.content_size().height)
    }

    fn modifiers(&self) -> Modifiers {
        let modifiers = NSEvent::modifierFlags_class();

        let control = modifiers.contains(NSEventModifierFlags::Control);
        let alt = modifiers.contains(NSEventModifierFlags::Option);
        let shift = modifiers.contains(NSEventModifierFlags::Shift);
        let command = modifiers.contains(NSEventModifierFlags::Command);
        let function = modifiers.contains(NSEventModifierFlags::Function);

        Modifiers {
            control,
            alt,
            shift,
            platform: command,
            function,
        }
    }

    fn capslock(&self) -> Capslock {
        let modifiers = NSEvent::modifierFlags_class();

        Capslock {
            on: modifiers.contains(NSEventModifierFlags::CapsLock),
        }
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.0.as_ref().lock().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0.as_ref().lock().input_handler.take()
    }

    fn prompt(
        &self,
        level: PromptLevel,
        msg: &str,
        detail: Option<&str>,
        answers: &[PromptButton],
    ) -> Option<oneshot::Receiver<usize>> {
        // macOs applies overrides to modal window buttons after they are added.
        // Two most important for this logic are:
        // * Buttons with "Cancel" title will be displayed as the last buttons in the modal
        // * Last button added to the modal via `addButtonWithTitle` stays focused
        // * Focused buttons react on "space"/" " keypresses
        // * Usage of `keyEquivalent`, `makeFirstResponder` or `setInitialFirstResponder` does not change the focus
        //
        // See also https://developer.apple.com/documentation/appkit/nsalert/1524532-addbuttonwithtitle#discussion
        // ```
        // By default, the first button has a key equivalent of Return,
        // any button with a title of “Cancel” has a key equivalent of Escape,
        // and any button with the title “Don’t Save” has a key equivalent of Command-D (but only if it’s not the first button).
        // ```
        //
        // To avoid situations when the last element added is "Cancel" and it gets the focus
        // (hence stealing both ESC and Space shortcuts), we find and add one non-Cancel button
        // last, so it gets focus and a Space shortcut.
        // This way, "Save this file? Yes/No/Cancel"-ish modals will get all three buttons mapped with a key.
        use objc2_foundation::{NSInteger, NSString};

        let initial_focus_ix = answers
            .iter()
            .enumerate()
            .rev()
            .find(|(_, label)| !label.is_cancel())
            .map(|(ix, _)| ix)
            .filter(|&ix| ix > 0);

        let marker = MainThreadMarker::new().expect("alert not on main thread");
        let alert = NSAlert::new(marker);
        alert.setAlertStyle(match level {
            PromptLevel::Critical => NSAlertStyle::Critical,
            PromptLevel::Warning => NSAlertStyle::Warning,
            PromptLevel::Info => NSAlertStyle::Informational,
        });
        let message = NSString::from_str(msg);
        alert.setMessageText(message.as_ref());

        if let Some(detail) = detail {
            let detail_text = NSString::from_str(detail);
            alert.setInformativeText(detail_text.as_ref());
        }

        let mut initial_focus_button: Option<Retained<NSButton>> = None;
        for (ix, answer) in answers.iter().enumerate() {
            let title = NSString::from_str(answer.label());
            let button = alert.addButtonWithTitle(&title);
            button.setTag(ix as NSInteger);

            if answer.is_cancel() {
                if let Some(key) = core::char::from_u32(ESCAPE_KEY) {
                    let key = NSString::from_str(&key.to_string());
                    button.setKeyEquivalent(&key);
                }
            } else if Some(ix) == initial_focus_ix {
                initial_focus_button = Some(button);
            }
        }

        if let Some(button) = initial_focus_button {
            alert.window().setInitialFirstResponder(Some(&button));
        }

        let (done_tx, done_rx) = oneshot::channel();
        let done_tx = Cell::new(Some(done_tx));

        let block = RcBlock::new(move |answer: NSInteger| {
            if let Some(done_tx) = done_tx.take() {
                let _ = done_tx.send(answer.try_into().unwrap());
            }
        });

        let lock = self.0.lock();
        let native_window = lock.native_window.clone();
        let executor = lock.executor.clone();
        executor
            .spawn(async move {
                // SAFETY: `native_window` is an Objective-C `NSWindow` pointer
                // owned by the platform window; bridge it into objc2.
                alert.beginSheetModalForWindow_completionHandler(&native_window, Some(&block));
            })
            .detach();
        Some(done_rx)
    }

    fn activate(&self) {
        let window = self.0.lock().native_window.clone();
        let executor = self.0.lock().executor.clone();
        executor
            .spawn(async move {
                window.makeKeyAndOrderFront(None);
            })
            .detach();
    }

    fn is_active(&self) -> bool {
        self.0.lock().native_window.isKeyWindow()
    }

    // is_hovered is unused on macOS. See Window::is_window_hovered.
    fn is_hovered(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        let mtm = MainThreadMarker::new().expect("Must be on main thread");
        let app = NSApplication::sharedApplication(mtm);
        let window = self.0.lock().native_window.clone();
        let title = NSString::from_str(title);
        app.changeWindowsItem_title_filename(&window, &title, false);
        window.setTitle(&title);
        self.0.lock().move_traffic_light();
    }

    fn get_title(&self) -> String {
        self.0.lock().native_window.title().to_string()
    }

    fn set_app_id(&mut self, _app_id: &str) {}

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut this = self.0.as_ref().lock();
        this.background_appearance = background_appearance;

        let opaque = background_appearance == WindowBackgroundAppearance::Opaque;
        this.renderer.update_transparency(!opaque);

        this.native_window.setOpaque(opaque);
        let background_color = if opaque {
            NSColor::colorWithSRGBRed_green_blue_alpha(0f64, 0f64, 0f64, 1f64)
        } else {
            // Not using `+[NSColor clearColor]` to avoid broken shadow.
            NSColor::colorWithSRGBRed_green_blue_alpha(0f64, 0f64, 0f64, 0.0001)
        };
        this.native_window.setBackgroundColor(Some(&background_color));

        if background_appearance != WindowBackgroundAppearance::Blurred {
            if let Some(blur_view) = this.blurred_view.clone() {
                NSView::removeFromSuperview(&blur_view);
                this.blurred_view = None;
            }
        } else if this.blurred_view.is_none()
            && let Some(content_view) = this.native_window.contentView()
        {
            let mtm = MainThreadMarker::new().expect("Must run on the main thread");
            let frame = content_view.bounds();
            let blur_view = super::blurred_view::BlurredView::new(mtm, frame);
            blur_view.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            content_view.addSubview_positioned_relativeTo(&blur_view, NSWindowOrderingMode::Below, None);
            this.blurred_view = Some(blur_view);
        }
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.as_ref().lock().background_appearance
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        // TODO: we could ask wgpu here but we need access to WgpuContext
        false
    }

    fn set_edited(&mut self, edited: bool) {
        let this = self.0.lock();
        this.native_window.setDocumentEdited(edited);

        // Changing the document edited state resets the traffic light position,
        // so we have to move it again.
        this.move_traffic_light();
    }

    fn show_character_palette(&self) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                let mtm = MainThreadMarker::new().expect("Must be on main thread");
                let app = NSApplication::sharedApplication(mtm);
                app.orderFrontCharacterPalette(Some(&*window));
            })
            .detach();
    }

    fn minimize(&self) {
        self.0.lock().native_window.miniaturize(None);
    }

    fn zoom(&self) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                window.zoom(None);
            })
            .detach();
    }

    fn toggle_fullscreen(&self) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                window.toggleFullScreen(None);
            })
            .detach();
    }

    fn is_fullscreen(&self) -> bool {
        self.0
            .lock()
            .native_window
            .styleMask()
            .contains(NSWindowStyleMask::FullScreen)
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.as_ref().lock().request_frame_callback = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> crate::DispatchEventResult>) {
        self.0.as_ref().lock().event_callback = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.as_ref().lock().activate_callback = Some(callback);
    }

    fn on_hover_status_change(&self, _: Box<dyn FnMut(bool)>) {}

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.as_ref().lock().resize_callback = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().moved_callback = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.as_ref().lock().should_close_callback = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.as_ref().lock().close_callback = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {}

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().appearance_changed_callback = Some(callback);
    }

    fn tabbed_windows(&self) -> Option<Vec<SystemWindowTab>> {
        let windows = self.0.lock().native_window.tabbedWindows()?;

        let count = windows.count();
        let mut result = Vec::new();
        for i in 0..count {
            let window = windows.objectAtIndex(i);
            if let Some(state) = to_window_state(&window) {
                let handle = state.lock().handle;
                let title = window.title();
                let title = SharedString::from(title.to_string());
                result.push(SystemWindowTab::new(title, handle));
            }
        }

        Some(result)
    }

    fn tab_bar_visible(&self) -> bool {
        self.0
            .lock()
            .native_window
            .tabGroup()
            .map(|tg| tg.isTabBarVisible())
            .unwrap_or(false)
    }

    fn on_move_tab_to_new_window(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().move_tab_to_new_window_callback = Some(callback);
    }

    fn on_merge_all_windows(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().merge_all_windows_callback = Some(callback);
    }

    fn on_select_next_tab(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().select_next_tab_callback = Some(callback);
    }

    fn on_select_previous_tab(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().select_previous_tab_callback = Some(callback);
    }

    fn on_toggle_tab_bar(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().toggle_tab_bar_callback = Some(callback);
    }

    fn draw(&self, scene: &crate::Scene) {
        let mut this = self.0.lock();
        this.renderer.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.0.lock().renderer.sprite_atlas().clone()
    }

    fn gpu_specs(&self) -> Option<crate::GpuSpecs> {
        self.0.lock().renderer.gpu_specs().into()
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {
        let executor = self.0.lock().executor.clone();
        executor
            .spawn(async move {
                let mtm = MainThreadMarker::new().expect("Must be on main thread");
                let Some(input_context) = NSTextInputContext::currentInputContext(mtm) else {
                    return;
                };
                input_context.invalidateCharacterCoordinates();
            })
            .detach()
    }

    fn titlebar_double_click(&self) {
        let this = self.0.lock();
        let window = this.native_window.clone();
        this.executor
            .spawn(async move {
                let defaults = NSUserDefaults::standardUserDefaults();
                let domain = ns_string!("NSGlobalDomain");
                let key = ns_string!("AppleActionOnDoubleClick");

                let dict = defaults.persistentDomainForName(domain);
                let action = dict
                    .and_then(|dict| dict.objectForKey(key))
                    .and_then(|value| value.downcast::<NSString>().ok())
                    .map(|value| value.to_string())
                    .unwrap_or_default();

                match action.as_str() {
                    "None" => {
                        // "Do Nothing" selected, so do no action
                    }
                    "Minimize" => {
                        window.miniaturize(None);
                    }
                    "Maximize" => {
                        window.zoom(None);
                    }
                    "Fill" => {
                        // There is no documented API for "Fill" action, so we'll just zoom the window
                        window.zoom(None);
                    }
                    _ => {
                        window.zoom(None);
                    }
                }
            })
            .detach();
    }

    fn start_window_move(&self) {
        let window = self.0.lock().native_window.clone();
        let mtm = MainThreadMarker::new().expect("Must be on main thread");
        let app = NSApplication::sharedApplication(mtm);
        if let Some(event) = app.currentEvent() {
            window.performWindowDragWithEvent(&event);
        }
    }
}

impl rwh::HasWindowHandle for MacWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        // SAFETY: The AppKitWindowHandle is a wrapper around a pointer to an NSView
        let view = self.0.lock().native_view.clone();
        let view = NonNull::new(Retained::as_ptr(&view) as *mut c_void).unwrap();
        Ok(unsafe { rwh::WindowHandle::borrow_raw(rwh::RawWindowHandle::AppKit(rwh::AppKitWindowHandle::new(view))) })
    }
}

impl rwh::HasDisplayHandle for MacWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        // SAFETY: This is a no-op on macOS
        unsafe { Ok(rwh::DisplayHandle::borrow_raw(rwh::AppKitDisplayHandle::new().into())) }
    }
}

extern "C" fn step(view: *mut c_void) {
    if view.is_null() {
        return;
    }
    let view = unsafe { &*view.cast::<GPUIView>() };
    let state = view.window_state();

    let mut lock = state.lock();
    if let Some(mut callback) = lock.request_frame_callback.take() {
        drop(lock);
        callback(Default::default());
        state.lock().request_frame_callback = Some(callback);
    }
}

fn get_scale_factor(native_window: &NSWindow) -> f32 {
    let factor = {
        let Some(screen) = native_window.screen() else {
            return 2.0;
        };
        screen.backingScaleFactor() as f32
    };

    // We are not certain what triggers this, but it seems that sometimes
    // this method would return 0 (https://github.com/zed-industries/zed/issues/6412)
    // It seems most likely that this would happen if the window has no screen
    // (if it is off-screen), though we'd expect to see viewDidChangeBackingProperties before
    // it was rendered for real.
    // Regardless, attempt to avoid the issue here.
    if factor == 0.0 { 2. } else { factor }
}

// Update the window scale factor and drawable size, and call the resize callback if any.
pub(crate) fn update_window_scale_factor(window_state: &Arc<Mutex<MacWindowState>>) {
    let mut lock = window_state.as_ref().lock();
    let scale_factor = lock.scale_factor();
    let size = lock.content_size();
    let drawable_size = size.to_device_pixels(scale_factor);
    if let Some(layer) = lock.native_view.layer() {
        layer.setContentsScale(scale_factor as f64);
    }

    lock.renderer.update_drawable_size(drawable_size);

    if let Some(mut callback) = lock.resize_callback.take() {
        let content_size = lock.content_size();
        let scale_factor = lock.scale_factor();
        drop(lock);
        callback(content_size, scale_factor);
        window_state.as_ref().lock().resize_callback = Some(callback);
    };
}

pub(crate) async fn synthetic_drag(window_state: Weak<Mutex<MacWindowState>>, drag_id: usize, event: MouseMoveEvent) {
    loop {
        Timer::after(Duration::from_millis(16)).await;
        if let Some(window_state) = window_state.upgrade() {
            let mut lock = window_state.lock();
            if lock.synthetic_drag_counter == drag_id {
                if let Some(mut callback) = lock.event_callback.take() {
                    drop(lock);
                    callback(PlatformInput::MouseMove(event.clone()));
                    window_state.lock().event_callback = Some(callback);
                }
            } else {
                break;
            }
        }
    }
}

pub(crate) fn convert_mouse_position(position: NSPoint, window_height: Pixels) -> Point<Pixels> {
    point(
        px(position.x as f32),
        // macOS screen coordinates are relative to bottom left
        window_height - px(position.y as f32),
    )
}

fn drag_event_position(
    window_state: &Mutex<MacWindowState>,
    dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
) -> Point<Pixels> {
    let drag_location = dragging_info.draggingLocation();
    convert_mouse_position(drag_location, window_state.lock().content_size().height)
}

fn external_paths_from_event(dragging_info: &ProtocolObject<dyn NSDraggingInfo>) -> Option<ExternalPaths> {
    let mut paths = SmallVec::new();
    let pasteboard = dragging_info.draggingPasteboard();
    let classes = NSArray::from_slice(&[NSURL::class()]);
    let options = NSDictionary::new();
    let Some(urls) = (unsafe { pasteboard.readObjectsForClasses_options(&classes, Some(&options)) }) else {
        return None;
    };
    for file in urls {
        if let Some(url) = file.downcast::<NSURL>().ok()
            && let Some(s) = url.absoluteString()
        {
            paths.push(PathBuf::from(s.to_string()))
        }
    }
    Some(ExternalPaths(paths))
}

fn send_new_event(window_state_lock: &Mutex<MacWindowState>, e: PlatformInput) -> bool {
    let window_state = window_state_lock.lock().event_callback.take();
    if let Some(mut callback) = window_state {
        callback(e);
        window_state_lock.lock().event_callback = Some(callback);
        true
    } else {
        false
    }
}

#[allow(non_snake_case)]
pub(crate) fn is_macos_version_at_least(majorVersion: isize, minorVersion: isize, patchVersion: isize) -> bool {
    let min_version = NSOperatingSystemVersion {
        majorVersion,
        minorVersion,
        patchVersion,
    };
    NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(min_version)
}

pub(crate) fn remove_layer_background(layer: &CALayer) {
    layer.setBackgroundColor(None);

    let class_name = layer.className();
    if class_name.isEqualToString(ns_string!("CAChameleonLayer")) {
        // Remove the desktop tinting effect.
        layer.setHidden(true);
        return;
    }

    if let Some(filters) = layer.filters() {
        // Remove the increased saturation.
        // The effect of a `CAFilter` or `CIFilter` is determined by its name, and the
        // `description` reflects its name and some parameters. Currently `NSVisualEffectView`
        // uses a `CAFilter` named "colorSaturate". If one day they switch to `CIFilter`, the
        // `description` will still contain "Saturat" ("... inputSaturation = ...").
        let test_string = ns_string!("Saturat");
        let count = filters.count();
        for i in 0..count {
            let filter = filters.objectAtIndex(i);
            if filter.class().name() == c"CAFilter" || filter.class().name() == c"CIFilter" {
                let description: Retained<NSString> = unsafe { msg_send![&filter, description] };
                let hit = description.containsString(test_string);
                if !hit {
                    continue;
                }
            } else {
                continue;
            }

            let all_indices = NSRange {
                location: 0,
                length: count,
            };
            let indices = NSMutableIndexSet::new(); //: id = msg_send![class!(NSMutableIndexSet), indexSet];
            indices.addIndexesInRange(all_indices);
            indices.removeIndex(i);
            let filtered = filters.objectsAtIndexes(&indices);
            unsafe { layer.setFilters(Some(&filtered)) };
            break;
        }
    }

    let sublayers = unsafe { layer.sublayers() };
    if let Some(sublayers) = sublayers {
        let count = sublayers.count();
        for i in 0..count {
            let sublayer = sublayers.objectAtIndex(i);
            remove_layer_background(&sublayer);
        }
    }
}
