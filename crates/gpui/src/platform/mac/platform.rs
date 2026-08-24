use super::{MacKeyboardLayout, MacKeyboardMapper, events::key_to_native};
use crate::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, ForegroundExecutor, KeyContext, Keymap,
    MacDispatcher, MacDisplay, MacWindow, Menu, MenuItem, OsMenu, OwnedMenu, PathPromptOptions, Platform,
    PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem, PlatformWindow, Result,
    SystemMenuType, Task, WindowAppearance, WindowParams,
    platform::{mac::pasteboard::Pasteboard, wgpu::WgpuContext},
};
use anyhow::{Context, anyhow};
use block2::RcBlock;
use dispatch2::DispatchQueue;
use futures::channel::oneshot;
use itertools::Itertools;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSApplicationDelegateReply, NSCursor,
    NSCursorFrameResizeDirections, NSCursorFrameResizePosition, NSDocumentController, NSEventModifierFlags, NSMenu,
    NSMenuDelegate, NSMenuItem, NSMenuItemValidation, NSModalResponse, NSModalResponseOK, NSOpenPanel, NSResponder,
    NSSavePanel, NSScroller, NSScrollerStyle, NSVisualEffectState, NSWorkspace,
};
use objc2_core_foundation::{CFRunLoop, CFString};
use objc2_foundation::{
    NSArray, NSBundle, NSError, NSInteger, NSNotification, NSNotificationCenter, NSNumber, NSObjectProtocol,
    NSProcessInfo, NSString, NSURL, NSUserDefaults, ns_string,
};
use parking_lot::Mutex;
use semver::Version as SemanticVersion;
use std::{
    cell::Cell,
    ffi::{CStr, OsStr, c_void},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::{self, NonNull},
    rc::Rc,
    str,
    sync::{Arc, OnceLock},
};
use util::{
    ResultExt,
    command::{new_smol_command, new_std_command},
};

#[derive(Default)]
struct GPUIAppDelegateIvars {
    platform: *const MacPlatform,
}

define_class!(
    #[unsafe(super(NSResponder))]
    #[name = "GPUIAppDelegate"]
    #[ivars = GPUIAppDelegateIvars]
        #[thread_kind = MainThreadOnly]
    struct GPUIAppDelegate;

    unsafe impl NSObjectProtocol for GPUIAppDelegate {}

    unsafe impl NSApplicationDelegate for GPUIAppDelegate {
        #[unsafe(method(applicationWillFinishLaunching:))]
        fn will_finish_launching(&self, notification: &NSNotification) {
            let user_defaults = NSUserDefaults::standardUserDefaults();

            // The autofill heuristic controller causes slowdown and high CPU usage.
            // We don't know exactly why. This disables the full heuristic controller.
            //
            // Adapted from: https://github.com/ghostty-org/ghostty/pull/8625
            let name = ns_string!("NSAutoFillHeuristicControllerEnabled");
            if let Some(_existing_value) = user_defaults.objectForKey(name) {
                let false_value = NSNumber::numberWithBool(false);
                unsafe { user_defaults.setObject_forKey(Some(&false_value), name) };
            }
        }

        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let app = NSApplication::sharedApplication(MainThreadMarker::from(self));
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

            let notification_center = NSNotificationCenter::defaultCenter();
            let name = ns_string!("NSTextInputContextKeyboardSelectionDidChangeNotification");
            unsafe {
            notification_center.addObserver_selector_name_object(self, sel!(onKeyboardLayoutChange:), Some(name), None)
            };

            let callback = self.platform().0.lock().finish_launching.take();
            if let Some(callback) = callback {
                callback();
            }
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _notification: &NSNotification, has_open_windows: bool) {
            if !has_open_windows {
                let platform = self.platform();
                let mut lock = platform.0.lock();
                if let Some(mut callback) = lock.reopen.take() {
                    drop(lock);
                    callback();
                    platform.0.lock().reopen.get_or_insert(callback);
                }
            }
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            let platform = self.platform();
            let mut lock = platform.0.lock();
            if let Some(mut callback) = lock.quit.take() {
                drop(lock);
                callback();
                platform.0.lock().quit.get_or_insert(callback);
            }
        }

        #[unsafe(method(applicationDockMenu:))]
        fn handle_dock_menu(&self, sender: &NSApplication) -> *mut AnyObject {
            let platform = self.platform();
            let mut lock = platform.0.lock();
            if let Some(menu) = lock.dock_menu.as_ref() {
                Retained::as_ptr(menu) as *mut AnyObject
            } else {
                std::ptr::null_mut()
            }
        }

        #[unsafe(method(application:openURLs:))]
        fn open_urls(&self, app: &NSApplication, urls: &NSArray<NSURL>) {
            let urls = (0..urls.count())
                    .filter_map(|i| {
                        let url = urls.objectAtIndex(i);
                        url.absoluteString().map(|s| s.to_string())
                    })
                    .collect::<Vec<_>>();
            self.dispatch_open_urls(urls);
        }

        #[unsafe(method(application:openFiles:))]
        fn open_files(&self, app: &NSApplication, filenames: &NSArray<NSString>) {
            let urls =(0..filenames.count())
                .map(|i| {
                    let file = filenames.objectAtIndex(i);
                    file.to_string()
                })
                .collect::<Vec<_>>();
            self.dispatch_open_urls(urls);
            app.replyToOpenOrPrint(NSApplicationDelegateReply::Success);
        }

        #[unsafe(method(application:openFile:))]
        fn open_file(&self, app: &NSApplication, filename: &NSString) -> bool {
            self.dispatch_open_urls(vec![filename.to_string()])
        }

    }

    unsafe impl NSMenuItemValidation for GPUIAppDelegate {
        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, item: &NSMenuItem) -> bool {
            let mut result = false;
            let platform = self.platform();
            let mut lock = platform.0.lock();
            if let Some(mut callback) = lock.validate_menu_command.take() {
                let tag = item.tag();
                let index = tag as usize;
                if let Some(action) = lock.menu_actions.get(index) {
                    let action = action.boxed_clone();
                    drop(lock);
                    result = callback(action.as_ref());
                }
                platform
                    .0
                    .lock()
                    .validate_menu_command
                    .get_or_insert(callback);
            }
            result
        }
    }

    unsafe impl NSMenuDelegate for GPUIAppDelegate {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, menu: &NSMenu) {
            let platform = self.platform();
            let mut lock = platform.0.lock();
            if let Some(mut callback) = lock.will_open_menu.take() {
                drop(lock);
                callback();
                platform.0.lock().will_open_menu.get_or_insert(callback);
            }
        }
    }

    impl GPUIAppDelegate {
        // Add menu item handlers so that OS save panels have the correct key commands
        #[unsafe(method(handleGPUIMenuItem:))]
        fn handle_gpui_menu_item(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(cut:))]
        fn handle_cut(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(copy:))]
        fn handle_copy(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(paste:))]
        fn handle_paste(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(selectAll:))]
        fn handle_select_all(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(undo:))]
        fn handle_undo(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(redo:))]
        fn handle_redo(&self, item: &AnyObject) {
            self.handle_menu_item(item)
        }

        #[unsafe(method(onKeyboardLayoutChange:))]
        fn on_keyboard_layout_change(&self, _: Option<&AnyObject>) {
            let platform = self.platform();
            let mut lock = platform.0.lock();
            let keyboard_layout = MacKeyboardLayout::new();
            lock.keyboard_mapper = Rc::new(MacKeyboardMapper::new(keyboard_layout.id()));
            if let Some(mut callback) = lock.on_keyboard_layout_change.take() {
                drop(lock);
                callback();
                platform
                    .0
                    .lock()
                    .on_keyboard_layout_change
                    .get_or_insert(callback);
            }
        }
    }
);

impl GPUIAppDelegate {
    fn new(mtm: MainThreadMarker, platform: *const MacPlatform) -> Retained<Self> {
        let this = Self::alloc(mtm);
        let this = this.set_ivars(GPUIAppDelegateIvars { platform });
        unsafe { msg_send![super(this), init] }
    }

    fn platform(&self) -> &MacPlatform {
        let platform = self.ivars().platform;
        unsafe { &*platform }
    }

    fn handle_menu_item(&self, sender: &AnyObject) {
        let Some(item) = sender.downcast_ref::<NSMenuItem>() else {
            return;
        };
        let platform = self.platform();
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.menu_command.take() {
            let index = item.tag() as usize;
            if let Some(action) = lock.menu_actions.get(index) {
                let action = action.boxed_clone();
                drop(lock);
                callback(&*action);
            }
            platform.0.lock().menu_command.get_or_insert(callback);
        }
    }

    fn dispatch_open_urls(&self, urls: Vec<String>) -> bool {
        let platform = self.platform();
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.open_urls.take() {
            drop(lock);
            callback(urls);
            platform.0.lock().open_urls.get_or_insert(callback);
            true
        } else {
            false
        }
    }
}

pub(crate) struct MacPlatform(Mutex<MacPlatformState>);

pub(crate) struct MacPlatformState {
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<dyn PlatformTextSystem>,
    pub renderer_context: Option<WgpuContext>,
    headless: bool,
    general_pasteboard: Pasteboard,
    find_pasteboard: Pasteboard,
    reopen: Option<Box<dyn FnMut()>>,
    on_keyboard_layout_change: Option<Box<dyn FnMut()>>,
    quit: Option<Box<dyn FnMut()>>,
    menu_command: Option<Box<dyn FnMut(&dyn Action)>>,
    validate_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    will_open_menu: Option<Box<dyn FnMut()>>,
    menu_actions: Vec<Box<dyn Action>>,
    open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    finish_launching: Option<Box<dyn FnOnce()>>,
    dock_menu: Option<Retained<NSMenu>>,
    menus: Option<Vec<OwnedMenu>>,
    keyboard_mapper: Rc<MacKeyboardMapper>,
}

impl Default for MacPlatform {
    fn default() -> Self {
        Self::new(false)
    }
}

impl MacPlatform {
    pub(crate) fn new(headless: bool) -> Self {
        let dispatcher = Arc::new(MacDispatcher);

        let text_system: Arc<dyn PlatformTextSystem> = if headless {
            Arc::new(crate::NoopTextSystem::new())
        } else {
            #[cfg(feature = "cosmic-text")]
            let text_system = Arc::new(crate::CosmicTextSystem::new("System Font"));
            #[cfg(not(feature = "cosmic-text"))]
            let text_system = Arc::new(crate::NoopTextSystem::new());
            text_system
        };

        let keyboard_layout = MacKeyboardLayout::new();
        let keyboard_mapper = Rc::new(MacKeyboardMapper::new(keyboard_layout.id()));

        Self(Mutex::new(MacPlatformState {
            headless,
            text_system,
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            renderer_context: if headless {
                None
            } else {
                Some(WgpuContext::new().context("Unable to init GPU context").unwrap())
            },
            general_pasteboard: Pasteboard::general(),
            find_pasteboard: Pasteboard::find(),
            reopen: None,
            quit: None,
            menu_command: None,
            validate_menu_command: None,
            will_open_menu: None,
            menu_actions: Default::default(),
            open_urls: None,
            finish_launching: None,
            dock_menu: None,
            on_keyboard_layout_change: None,
            menus: None,
            keyboard_mapper,
        }))
    }

    pub fn new_background_executor() -> BackgroundExecutor {
        BackgroundExecutor::new(Arc::new(MacDispatcher))
    }

    fn create_menu_bar(
        &self,
        mtm: MainThreadMarker,
        menus: &Vec<Menu>,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenu> {
        let application_menu = NSMenu::new(mtm);
        application_menu.setDelegate(Some(delegate));

        for menu_config in menus {
            let menu = NSMenu::new(mtm);
            let menu_title = NSString::from_str(&menu_config.name);
            menu.setTitle(&menu_title);
            menu.setDelegate(Some(delegate));

            for item_config in &menu_config.items {
                menu.addItem(&self.create_menu_item(mtm, item_config, delegate, actions, keymap));
            }

            let menu_item = NSMenuItem::new(mtm);
            menu_item.setTitle(&menu_title);
            menu_item.setSubmenu(Some(&*menu));
            application_menu.addItem(&menu_item);

            if menu_config.name == "Window" {
                let app = NSApplication::sharedApplication(mtm);
                app.setWindowsMenu(Some(&*menu));
            }
        }

        application_menu
    }

    fn create_dock_menu(
        &self,
        mtm: MainThreadMarker,
        menu_items: Vec<MenuItem>,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenu> {
        let dock_menu = NSMenu::new(mtm);
        dock_menu.setDelegate(Some(delegate));
        for item_config in menu_items {
            let item = self.create_menu_item(mtm, &item_config, delegate, actions, keymap);
            dock_menu.addItem(&item);
        }

        dock_menu
    }

    fn create_menu_item(
        &self,
        mtm: MainThreadMarker,
        item: &MenuItem,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenuItem> {
        static DEFAULT_CONTEXT: OnceLock<Vec<KeyContext>> = OnceLock::new();

        unsafe {
            match item {
                MenuItem::Separator => NSMenuItem::separatorItem(mtm),
                MenuItem::Action {
                    name,
                    action,
                    os_action,
                    checked,
                } => {
                    // Note that this is intentionally using earlier bindings, whereas typically
                    // later ones take display precedence. See the discussion on
                    // https://github.com/zed-industries/zed/issues/23621
                    let keystrokes = keymap
                        .bindings_for_action(action.as_ref())
                        .find_or_first(|binding| {
                            binding.predicate().is_none_or(|predicate| {
                                predicate.eval(DEFAULT_CONTEXT.get_or_init(|| {
                                    let mut workspace_context = KeyContext::new_with_defaults();
                                    workspace_context.add("Workspace");
                                    let mut pane_context = KeyContext::new_with_defaults();
                                    pane_context.add("Pane");
                                    let mut editor_context = KeyContext::new_with_defaults();
                                    editor_context.add("Editor");

                                    pane_context.extend(&editor_context);
                                    workspace_context.extend(&pane_context);
                                    vec![workspace_context]
                                }))
                            })
                        })
                        .map(|binding| binding.keystrokes());

                    let selector = match os_action {
                        Some(crate::OsAction::Cut) => sel!(cut:),
                        Some(crate::OsAction::Copy) => sel!(copy:),
                        Some(crate::OsAction::Paste) => sel!(paste:),
                        Some(crate::OsAction::SelectAll) => sel!(selectAll:),
                        // "undo:" and "redo:" are always disabled in our case, as
                        // we don't have a NSTextView/NSTextField to enable them on.
                        Some(crate::OsAction::Undo) => sel!(handleGPUIMenuItem:),
                        Some(crate::OsAction::Redo) => sel!(handleGPUIMenuItem:),
                        None => sel!(handleGPUIMenuItem:),
                    };
                    let name_str = NSString::from_str(&name);

                    let item;
                    if let Some(keystrokes) = keystrokes {
                        if keystrokes.len() == 1 {
                            let keystroke = &keystrokes[0];
                            let mut mask = NSEventModifierFlags::empty();
                            for (modifier, flag) in &[
                                (keystroke.modifiers().platform, NSEventModifierFlags::Command),
                                (keystroke.modifiers().control, NSEventModifierFlags::Control),
                                (keystroke.modifiers().alt, NSEventModifierFlags::Option),
                                (keystroke.modifiers().shift, NSEventModifierFlags::Shift),
                            ] {
                                if *modifier {
                                    mask |= *flag;
                                }
                            }

                            let char_code = NSString::from_str(key_to_native(keystroke.key()).as_ref());
                            item = NSMenuItem::initWithTitle_action_keyEquivalent(
                                NSMenuItem::alloc(mtm),
                                &name_str,
                                Some(selector),
                                &char_code,
                            );
                            if Self::os_version() >= SemanticVersion::new(12, 0, 0) {
                                item.setAllowsAutomaticKeyEquivalentLocalization(false);
                            }
                            item.setKeyEquivalentModifierMask(mask);
                        } else {
                            item = NSMenuItem::initWithTitle_action_keyEquivalent(
                                NSMenuItem::alloc(mtm),
                                &name_str,
                                Some(selector),
                                ns_string!(""),
                            );
                        }
                    } else {
                        item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(mtm),
                            &name_str,
                            Some(selector),
                            ns_string!(""),
                        );
                    }

                    if *checked {
                        item.setState(NSVisualEffectState::Active.0);
                    }

                    let tag = actions.len();
                    item.setTag(tag as NSInteger);
                    actions.push(action.boxed_clone());
                    item
                }
                MenuItem::Submenu(Menu { name, items }) => {
                    let item = NSMenuItem::new(mtm);
                    let submenu = NSMenu::new(mtm);
                    submenu.setDelegate(Some(delegate));
                    for item in items {
                        submenu.addItem(&self.create_menu_item(mtm, item, delegate, actions, keymap));
                    }
                    item.setSubmenu(Some(&*submenu));
                    item.setTitle(&NSString::from_str(&name));
                    item
                }
                MenuItem::SystemMenu(OsMenu { name, menu_type }) => {
                    let item = NSMenuItem::new(mtm);
                    let submenu = NSMenu::new(mtm);
                    submenu.setDelegate(Some(delegate));
                    item.setSubmenu(Some(&*submenu));
                    item.setTitle(&NSString::from_str(&name));

                    match menu_type {
                        SystemMenuType::Services => {
                            let app = NSApplication::sharedApplication(mtm);
                            app.setServicesMenu(Some(&*submenu));
                        }
                    }

                    item
                }
            }
        }
    }

    fn os_version() -> SemanticVersion {
        let process_info = NSProcessInfo::processInfo();
        let version = process_info.operatingSystemVersion();
        SemanticVersion::new(
            version.majorVersion as u64,
            version.minorVersion as u64,
            version.patchVersion as u64,
        )
    }
}

impl Platform for MacPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.0.lock().background_executor.clone()
    }

    fn foreground_executor(&self) -> crate::ForegroundExecutor {
        self.0.lock().foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.0.lock().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let mut state = self.0.lock();
        if state.headless {
            drop(state);
            on_finish_launching();
            CFRunLoop::run();
        } else {
            state.finish_launching = Some(on_finish_launching);
            drop(state);

            let app = NSApplication::sharedApplication(mtm);
            let app_delegate = GPUIAppDelegate::new(mtm, self);
            app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));
            app.run();
        }
    }

    fn quit(&self) {
        // Quitting the app causes us to close windows, which invokes `Window::on_close` callbacks
        // synchronously before this method terminates. If we call `Platform::quit` while holding a
        // borrow of the app state (which most of the time we will do), we will end up
        // double-borrowing the app state in the `on_close` callbacks for our open windows. To solve
        // this, we make quitting the application asynchronous so that we aren't holding borrows to
        // the app state on the stack when we actually terminate the app.

        unsafe {
            DispatchQueue::main().exec_async_f(ptr::null_mut(), quit);
        }

        extern "C" fn quit(_: *mut c_void) {
            let mtm = MainThreadMarker::new().unwrap();
            let app = NSApplication::sharedApplication(mtm);
            app.terminate(None);
        }
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {
        use std::os::unix::process::CommandExt as _;

        let app_pid = std::process::id().to_string();
        let app_path = self
            .app_path()
            .ok()
            // When the app is not bundled, `app_path` returns the
            // directory containing the executable. Disregard this
            // and get the path to the executable itself.
            .and_then(|path| (path.extension()?.to_str()? == "app").then_some(path))
            .unwrap_or_else(|| std::env::current_exe().unwrap());

        // Wait until this process has exited and then re-open this path.
        let script = r#"
            while kill -0 $0 2> /dev/null; do
                sleep 0.1
            done
            open "$1"
        "#;

        #[allow(
            clippy::disallowed_methods,
            reason = "We are restarting ourselves, using std command thus is fine"
        )]
        let restart_process = new_std_command("/bin/bash")
            .arg("-c")
            .arg(script)
            .arg(app_pid)
            .arg(app_path)
            .process_group(0)
            .spawn();

        match restart_process {
            Ok(_) => self.quit(),
            Err(e) => log::error!("failed to spawn restart script: {:?}", e),
        }
    }

    fn activate(&self) {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        app.activate();
    }

    fn hide(&self) {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        app.hide(None);
    }

    fn hide_other_apps(&self) {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        app.hideOtherApplications(None);
    }

    fn unhide_other_apps(&self) {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        app.unhideAllApplications(None);
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(MacDisplay::primary()))
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        MacDisplay::all().map(|screen| Rc::new(screen) as Rc<_>).collect()
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        MacWindow::active_window()
    }

    // Returns the windows ordered front-to-back, meaning that the active
    // window is the first one in the returned vec.
    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        Some(MacWindow::ordered_windows())
    }

    fn open_window(&self, handle: AnyWindowHandle, options: WindowParams) -> Result<Box<dyn PlatformWindow>> {
        let mut state = self.0.lock();

        let window = MacWindow::open(
            handle,
            options,
            state.foreground_executor.clone(),
            state.renderer_context.as_ref().unwrap(),
        );
        Ok(Box::new(window))
    }

    fn window_appearance(&self) -> WindowAppearance {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        let appearance = app.effectiveAppearance();
        WindowAppearance::from_native(&appearance)
    }

    fn open_url(&self, url: &str) {
        let str = NSString::from_str(url);
        let Some(ns_url) = NSURL::URLWithString(&str) else {
            log::error!("Failed to create NSURL from string: {}", url);
            return;
        };

        NSWorkspace::sharedWorkspace().openURL(&ns_url);
    }

    fn register_url_scheme(&self, scheme: &str) -> Task<anyhow::Result<()>> {
        // API only available post Monterey
        // https://developer.apple.com/documentation/appkit/nsworkspace/3753004-setdefaultapplicationaturl
        let (done_tx, done_rx) = oneshot::channel();
        if Self::os_version() < SemanticVersion::new(12, 0, 0) {
            return Task::ready(Err(anyhow!("macOS 12.0 or later is required to register URL schemes")));
        }

        let Some(bundle_id) = NSBundle::mainBundle().bundleIdentifier() else {
            return Task::ready(Err(anyhow!("Can only register URL scheme in bundled apps")));
        };

        let workspace = NSWorkspace::sharedWorkspace();
        let scheme = NSString::from_str(scheme);
        let Some(app) = workspace.URLForApplicationWithBundleIdentifier(&bundle_id) else {
            return Task::ready(Err(anyhow!("Cannot register URL scheme until app is installed")));
        };
        let done_tx = Cell::new(Some(done_tx));
        let block = RcBlock::new(move |error: *mut NSError| {
            let result = if error.is_null() {
                Ok(())
            } else {
                let msg = (unsafe { &*error as &NSError }).localizedDescription();
                Err(anyhow!("Failed to register: {msg:?}"))
            };

            if let Some(done_tx) = done_tx.take() {
                let _ = done_tx.send(result);
            }
        });
        workspace.setDefaultApplicationAtURL_toOpenURLsWithScheme_completionHandler(&app, &scheme, Some(&block));

        self.background_executor()
            .spawn(async { crate::Flatten::flatten(done_rx.await.map_err(|e| anyhow!(e))) })
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.0.lock().open_urls = Some(callback);
    }

    fn prompt_for_paths(&self, options: PathPromptOptions) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (done_tx, done_rx) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                unsafe {
                    let panel = NSOpenPanel::openPanel(MainThreadMarker::new().unwrap());
                    panel.setCanChooseDirectories(options.directories);
                    panel.setCanChooseFiles(options.files);
                    panel.setAllowsMultipleSelection(options.multiple);
                    panel.setCanCreateDirectories(true);
                    panel.setResolvesAliases(false);

                    let done_tx = Cell::new(Some(done_tx));
                    let panel2 = panel.clone();
                    let block = RcBlock::new(move |response: NSModalResponse| {
                        let result = if response == NSModalResponseOK {
                            let mut result = Vec::new();
                            let urls = panel2.URLs();
                            for i in 0..urls.count() {
                                let url = urls.objectAtIndex(i);
                                if url.isFileURL()
                                    && let Ok(path) = ns_url_to_path(&url)
                                {
                                    result.push(path)
                                }
                            }
                            Some(result)
                        } else {
                            None
                        };

                        if let Some(done_tx) = done_tx.take() {
                            let _ = done_tx.send(Ok(result));
                        }
                    });

                    if let Some(prompt) = options.prompt {
                        let ns_prompt = NSString::from_str(&prompt);
                        panel.setPrompt(Some(&*ns_prompt));
                    }

                    panel.beginWithCompletionHandler(&block);
                }
            })
            .detach();
        done_rx
    }

    fn prompt_for_new_path(
        &self,
        directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let directory = directory.to_owned();
        let suggested_name = suggested_name.map(|s| s.to_owned());
        let (done_tx, done_rx) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                let mtm = MainThreadMarker::new().unwrap();
                let panel = NSSavePanel::savePanel(mtm);
                let path = NSString::from_str(directory.to_string_lossy().as_ref());
                let url = NSURL::fileURLWithPath_isDirectory(&path, true);
                panel.setDirectoryURL(Some(&*url));

                if let Some(suggested_name) = suggested_name {
                    let name_string = NSString::from_str(&suggested_name);
                    panel.setNameFieldStringValue(&name_string);
                }

                let done_tx = Cell::new(Some(done_tx));
                let panel2 = panel.clone();
                let block = RcBlock::new(move |response: NSModalResponse| {
                    let mut result = None;
                    if response == NSModalResponseOK {
                        let Some(url) = panel2.URL() else {
                            return;
                        };
                        if url.isFileURL() {
                            result = (unsafe { ns_url_to_path(&url) }).ok().map(|mut result| {
                                let Some(filename) = result.file_name() else {
                                    return result;
                                };
                                let chunks = filename.as_bytes().split(|&b| b == b'.').collect::<Vec<_>>();

                                // https://github.com/zed-industries/zed/issues/16969
                                // Workaround a bug in macOS Sequoia that adds an extra file-extension
                                // sometimes. e.g. `a.sql` becomes `a.sql.s` or `a.txtx` becomes `a.txtx.txt`
                                //
                                // This is conditional on OS version because I'd like to get rid of it, so that
                                // you can manually create a file called `a.sql.s`. That said it seems better
                                // to break that use-case than breaking `a.sql`.
                                if chunks.len() == 3
                                    && chunks[1].starts_with(chunks[2])
                                    && Self::os_version() >= SemanticVersion::new(15, 0, 0)
                                {
                                    let new_filename = OsStr::from_bytes(
                                        &filename.as_bytes()[..chunks[0].len() + 1 + chunks[1].len()],
                                    )
                                    .to_owned();
                                    result.set_file_name(&new_filename);
                                }
                                result
                            })
                        }
                    }

                    if let Some(done_tx) = done_tx.take() {
                        let _ = done_tx.send(Ok(result));
                    }
                });
                panel.beginWithCompletionHandler(&block);
            })
            .detach();

        done_rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        true
    }

    fn reveal_path(&self, path: &Path) {
        let path = path.to_path_buf();
        self.0
            .lock()
            .background_executor
            .spawn(async move {
                let full_path = path.to_str().map(NSString::from_str);
                let root_full_path = ns_string!("");
                let workspace = NSWorkspace::sharedWorkspace();
                workspace.selectFile_inFileViewerRootedAtPath(full_path.as_deref(), root_full_path);
            })
            .detach();
    }

    fn open_with_system(&self, path: &Path) {
        let path = path.to_owned();
        self.0
            .lock()
            .background_executor
            .spawn(async move {
                if let Some(mut child) = new_smol_command("open")
                    .arg(path)
                    .spawn()
                    .context("invoking open command")
                    .log_err()
                {
                    child.status().await.log_err();
                }
            })
            .detach();
    }

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().reopen = Some(callback);
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_keyboard_layout_change = Some(callback);
    }

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.0.lock().menu_command = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().will_open_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.0.lock().validate_menu_command = Some(callback);
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(MacKeyboardLayout::new())
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        self.0.lock().keyboard_mapper.clone()
    }

    fn app_path(&self) -> Result<PathBuf> {
        let bundle = NSBundle::mainBundle();
        Ok(path_from_objc(&bundle.bundlePath()))
    }

    fn set_menus(&self, menus: Vec<Menu>, keymap: &Keymap) {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let mut state = self.0.lock();
        let actions = &mut state.menu_actions;
        let delegate = app.delegate().unwrap();
        let menu_delegate: Retained<ProtocolObject<dyn NSMenuDelegate>> = unsafe { Retained::cast_unchecked(delegate) };
        let menu = self.create_menu_bar(mtm, &menus, &menu_delegate, actions, keymap);
        drop(state);
        app.setMainMenu(Some(&*menu));
        self.0.lock().menus = Some(menus.into_iter().map(|menu| menu.owned()).collect());
    }

    fn get_menus(&self) -> Option<Vec<OwnedMenu>> {
        self.0.lock().menus.clone()
    }

    fn set_dock_menu(&self, menu: Vec<MenuItem>, keymap: &Keymap) {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let mut state = self.0.lock();
        let actions = &mut state.menu_actions;
        let delegate = app.delegate().unwrap();
        let menu_delegate: Retained<ProtocolObject<dyn NSMenuDelegate>> = unsafe { Retained::cast_unchecked(delegate) };
        let new = self.create_dock_menu(mtm, menu, &menu_delegate, actions, keymap);
        let _ = state.dock_menu.replace(new);
    }

    fn add_recent_document(&self, path: &Path) {
        let mtm = MainThreadMarker::new().unwrap();
        if let Some(path_str) = path.to_str() {
            let document_controller = NSDocumentController::sharedDocumentController(mtm);
            let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
            document_controller.noteNewRecentDocumentURL(&url);
        }
    }

    fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        let bundle = NSBundle::mainBundle();
        let name = NSString::from_str(name);
        let url = bundle.URLForAuxiliaryExecutable(&name).expect("resource not found");
        unsafe { ns_url_to_path(&url) }
    }

    /// Match cursor style to one of the styles available
    /// in macOS's [NSCursor](https://developer.apple.com/documentation/appkit/nscursor).
    fn set_cursor_style(&self, style: CursorStyle) {
        if style == CursorStyle::None {
            NSCursor::setHiddenUntilMouseMoves(true);
            return;
        }

        #[allow(deprecated)]
        let new_cursor = match style {
            CursorStyle::Arrow => NSCursor::arrowCursor(),
            CursorStyle::IBeam => NSCursor::IBeamCursor(),
            CursorStyle::Crosshair => NSCursor::crosshairCursor(),
            CursorStyle::ClosedHand => NSCursor::closedHandCursor(),
            CursorStyle::OpenHand => NSCursor::openHandCursor(),
            CursorStyle::PointingHand => NSCursor::pointingHandCursor(),
            CursorStyle::ResizeLeftRight => NSCursor::resizeLeftRightCursor(),
            CursorStyle::ResizeUpDown => NSCursor::resizeUpDownCursor(),
            CursorStyle::ResizeLeft => NSCursor::resizeLeftCursor(),
            CursorStyle::ResizeRight => NSCursor::resizeRightCursor(),
            CursorStyle::ResizeColumn => NSCursor::resizeLeftRightCursor(),
            CursorStyle::ResizeRow => NSCursor::resizeUpDownCursor(),
            CursorStyle::ResizeUp => NSCursor::resizeUpCursor(),
            CursorStyle::ResizeDown => NSCursor::resizeDownCursor(),
            CursorStyle::ResizeUpLeftDownRight => NSCursor::frameResizeCursorFromPosition_inDirections(
                NSCursorFrameResizePosition::TopLeft,
                NSCursorFrameResizeDirections::All,
            ),
            CursorStyle::ResizeUpRightDownLeft => NSCursor::frameResizeCursorFromPosition_inDirections(
                NSCursorFrameResizePosition::TopRight,
                NSCursorFrameResizeDirections::All,
            ),
            CursorStyle::IBeamCursorForVerticalLayout => NSCursor::IBeamCursorForVerticalLayout(),
            CursorStyle::OperationNotAllowed => NSCursor::operationNotAllowedCursor(),
            CursorStyle::DragLink => NSCursor::dragLinkCursor(),
            CursorStyle::DragCopy => NSCursor::dragCopyCursor(),
            CursorStyle::ContextualMenu => NSCursor::contextualMenuCursor(),
            CursorStyle::None => unreachable!(),
        };

        let old_cursor = NSCursor::currentCursor();
        if new_cursor != old_cursor {
            new_cursor.set();
        }
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        let style = NSScroller::preferredScrollerStyle(MainThreadMarker::new().unwrap());
        style == NSScrollerStyle::Overlay
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        let state = self.0.lock();
        state.general_pasteboard.read()
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        let state = self.0.lock();
        state.general_pasteboard.write(item);
    }

    fn read_from_find_pasteboard(&self) -> Option<ClipboardItem> {
        let state = self.0.lock();
        state.find_pasteboard.read()
    }

    fn write_to_find_pasteboard(&self, item: ClipboardItem) {
        let state = self.0.lock();
        state.find_pasteboard.write(item);
    }
}

fn path_from_objc(path: &NSString) -> PathBuf {
    PathBuf::from(&path.to_string())
}

unsafe fn ns_url_to_path(url: &NSURL) -> Result<PathBuf> {
    let path = url.fileSystemRepresentation();
    Ok(PathBuf::from(OsStr::from_bytes(unsafe {
        CStr::from_ptr(NonNull::as_ptr(path)).to_bytes()
    })))
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub(super) fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut AnyObject;
    pub(super) fn TISGetInputSourceProperty(inputSource: *mut AnyObject, propertyKey: *const c_void) -> *mut AnyObject;

    pub(super) fn UCKeyTranslate(
        keyLayoutPtr: *const ::std::os::raw::c_void,
        virtualKeyCode: u16,
        keyAction: u16,
        modifierKeyState: u32,
        keyboardType: u32,
        keyTranslateOptions: u32,
        deadKeyState: *mut u32,
        maxStringLength: usize,
        actualStringLength: *mut usize,
        unicodeString: *mut u16,
    ) -> u32;
    pub(super) fn LMGetKbdType() -> u16;
    pub(super) static kTISPropertyUnicodeKeyLayoutData: *const CFString;
    pub(super) static kTISPropertyInputSourceID: *const CFString;
    pub(super) static kTISPropertyLocalizedName: *const CFString;
}
