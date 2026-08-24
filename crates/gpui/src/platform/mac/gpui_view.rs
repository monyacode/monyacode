use crate::{
    KeyDownEvent, MacWindowState, Modifiers, ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, PlatformInput, PlatformInputHandler, Point, Size, point, px, synthetic_drag,
    update_window_scale_factor,
};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Sel},
};
use objc2_app_kit::{NSEvent, NSTextInputClient, NSView, NSWindowStyleMask};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSNotFound, NSObjectProtocol, NSPoint, NSRange, NSRangePointer,
    NSRect, NSSize, NSString,
};
use objc2_quartz_core::{CALayer, CALayerDelegate};
use std::{cell::RefCell, ops::Range, sync::Arc};

use parking_lot::Mutex;

#[derive(Default)]
pub struct GPUIViewIvars {
    window_state: RefCell<Option<Arc<Mutex<MacWindowState>>>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "GPUIView"]
    #[ivars = GPUIViewIvars]
    #[thread_kind = MainThreadOnly]
    pub struct GPUIView;

    unsafe impl CALayerDelegate for GPUIView {
        #[unsafe(method(displayLayer:))]
        fn display_layer(&self, _: &CALayer) {
            let window_state = self.window_state();
            let mut lock = window_state.lock();
            if let Some(mut callback) = lock.request_frame_callback.take() {
                lock.renderer.set_presents_with_transaction(true);
                lock.stop_display_link();
                drop(lock);
                callback(Default::default());

                let mut lock = window_state.lock();
                lock.request_frame_callback = Some(callback);
                lock.renderer.set_presents_with_transaction(false);
                lock.start_display_link();
            }
        }
    }

    unsafe impl NSTextInputClient for GPUIView {
        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            let has_marked_text_result = self
                .with_input_handler(|input_handler| input_handler.marked_text_range())
                .flatten();

            has_marked_text_result.is_some()
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            let marked_range_result = self
                .with_input_handler(|input_handler| input_handler.marked_text_range())
                .flatten();
            marked_range_result.map_or(NSRange::new(NSNotFound as usize, 0), |range| range.into())
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn valid_attributes_for_marked_text(&self) -> *mut NSArray<NSAttributedStringKey> {
            let array: Retained<NSArray<NSAttributedStringKey>> = NSArray::from_retained_slice(&[]);
            Retained::autorelease_return(array)
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect_for_character_range(&self, range: NSRange, _: NSRangePointer) -> NSRect {
            let frame = self.get_window_frame();
            self.with_input_handler(|input_handler| {
                input_handler.bounds_for_range(nsrange_to_range(&range)?)
            })
            .flatten()
            .map_or(
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(0., 0.)),
                |bounds| {
                    NSRect::new(
                        NSPoint::new(
                            frame.origin.x + bounds.origin.x.0 as f64,
                            frame.origin.y + frame.size.height
                                - bounds.origin.y.0 as f64
                                - bounds.size.height.0 as f64,
                        ),
                        NSSize::new(bounds.size.width.0 as f64, bounds.size.height.0 as f64),
                    )
                },
            )
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            let selected_range_result = self
                .with_input_handler(|input_handler| input_handler.selected_text_range(false))
                .flatten();

            selected_range_result.map_or(NSRange::new(NSNotFound as usize, 0), |selection| {
                selection.range.into()
            })
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(
            &self,
            text: &AnyObject,
            selected_range: NSRange,
            replacement_range: NSRange,
        ) {
            let text = maybe_attributed_string(text).to_string();
            let selected_range = nsrange_to_range(&selected_range);
            let replacement_range = nsrange_to_range(&replacement_range);
            self.with_input_handler(|input_handler| {
                input_handler.replace_and_mark_text_in_range(
                    replacement_range,
                    &text,
                    selected_range,
                )
            });
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            self.with_input_handler(|input_handler| input_handler.unmark_text());
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, string: &AnyObject, replacement_range: NSRange) {
            let string = maybe_attributed_string(string).to_string();
            let replacement_range = nsrange_to_range(&replacement_range);
            self.with_input_handler(|input_handler| {
                input_handler.replace_text_in_range(replacement_range, &string)
            });
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring_for_proposed_range(
            &self,
            range: NSRange,
            actual_range: NSRangePointer,
        ) -> *mut NSAttributedString {
            self.with_input_handler(|input_handler| {
                let range = nsrange_to_range(&range)?;
                if range.is_empty() {
                    return None;
                }
                let mut adjusted: Option<Range<usize>> = None;

                let selected_text = input_handler.text_for_range(range.clone(), &mut adjusted)?;
                if let Some(adjusted) = adjusted
                    && adjusted != range
                {
                    unsafe { actual_range.write(NSRange::from(adjusted)) };
                }
                let selected_text = NSString::from_str(&selected_text);
                let string = NSAttributedString::initWithString(
                    NSAttributedString::alloc(),
                    &selected_text,
                );
                Some(Retained::autorelease_return(string))
            })
            .flatten()
            .unwrap_or(std::ptr::null_mut())
        }

        // Suppress beep on keystrokes with modifier keys.
        #[unsafe(method(doCommandBySelector:))]
        fn do_command_by_selector(&self, _: Sel) {
            let state = self.window_state();
            let mut lock = state.as_ref().lock();
            let keystroke = lock.keystroke_for_do_command.take();
            let mut event_callback = lock.event_callback.take();
            drop(lock);

            if let Some((keystroke, mut callback)) = keystroke.zip(event_callback.as_mut()) {
                let prefer_character_input = keystroke.altgr && keystroke.prefer_character_input();
                let handled = (callback)(PlatformInput::KeyDown(KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input,
                }));
                state.as_ref().lock().do_command_handled = Some(!handled.propagate);
            }

            state.as_ref().lock().event_callback = event_callback;
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index_for_point(&self, position: NSPoint) -> u64 {
            let position = self.screen_point_to_gpui_point(position);
            self.with_input_handler(|input_handler| {
                input_handler.character_index_for_point(position)
            })
            .flatten()
            .map(|index| index as u64)
            .unwrap_or(NSNotFound as u64)
        }
    }
    unsafe impl NSObjectProtocol for GPUIView {}

    impl GPUIView {
        #[unsafe(method(performKeyEquivalent:))]
        fn handle_key_equivalent(&self, event: &NSEvent) -> bool {
            self.handle_key_event(event, true)
        }

        #[unsafe(method(keyDown:))]
        fn handle_key_down(&self, event: &NSEvent) {
            self.handle_key_event(event, false);
        }

        #[unsafe(method(keyUp:))]
        fn handle_key_up(&self, event: &NSEvent) {
            self.handle_key_event(event, false);
        }

        #[unsafe(method(mouseDown:))]
        fn handle_mouse_down(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(mouseUp:))]
        fn handle_mouse_up(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(rightMouseDown:))]
        fn handle_right_mouse_down(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(rightMouseUp:))]
        fn handle_right_mouse_up(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(otherMouseDown:))]
        fn handle_other_mouse_down(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(otherMouseUp:))]
        fn handle_other_mouse_up(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(mouseMoved:))]
        fn handle_mouse_moved(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(mouseExited:))]
        fn handle_mouse_exited(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(mouseDragged:))]
        fn handle_mouse_dragged(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(pressureChangeWithEvent:))]
        fn handle_pressure_change_with_event(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(scrollWheel:))]
        fn handle_scroll_wheel(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(swipeWithEvent:))]
        fn handle_swipe_with_event(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(flagsChanged:))]
        fn handle_flags_changed(&self, event: &NSEvent) {
            self.handle_view_event(event)
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            let state = self.window_state();
            let appearance_changed_callback = {
                let mut lock = state.as_ref().lock();
                lock.appearance_changed_callback.take()
            };
            if let Some(mut callback) = appearance_changed_callback {
                callback();
                state.lock().appearance_changed_callback = Some(callback);
            }
            state.lock().move_traffic_light();
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            let new_size = Size::<Pixels>::from(size);
            let old_size = Size::<Pixels>::from(self.frame().size);
            if old_size == new_size {
                return;
            }

            // bypass setFrameSize: for GPUIView
            unsafe { msg_send![super(self), setFrameSize: size] }

            let window_state = self.window_state();
            let mut lock = window_state.as_ref().lock();

            let scale_factor = lock.scale_factor();
            let drawable_size = new_size.to_device_pixels(scale_factor);
            lock.renderer.update_drawable_size(drawable_size);

            if let Some(mut callback) = lock.resize_callback.take() {
                let content_size = lock.content_size();
                let scale_factor = lock.scale_factor();
                drop(lock);
                callback(content_size, scale_factor);
                window_state.lock().resize_callback = Some(callback);
            };
        }

        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn view_did_change_effective_appearance(&self) {
            let state = self.window_state();
            update_window_scale_factor(&state);
            let mut lock = state.as_ref().lock();
            if let Some(mut callback) = lock.appearance_changed_callback.take() {
                drop(lock);
                callback();
                state.lock().appearance_changed_callback = Some(callback);
            }
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _: &AnyObject) -> bool {
            let state = self.window_state();
            let mut lock = state.as_ref().lock();
            lock.first_mouse = true;
            true
        }
    }
);

impl GPUIView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm);
        let this = this.set_ivars(GPUIViewIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    pub fn window_state(&self) -> Arc<Mutex<MacWindowState>> {
        self.ivars().window_state.borrow().as_ref().unwrap().clone()
    }

    pub fn set_window_state(&self, window_state: Arc<Mutex<MacWindowState>>) {
        self.ivars().window_state.replace(Some(window_state));
    }

    fn get_window_frame(&self) -> NSRect {
        let state = self.window_state();
        let lock = state.lock();
        let window = &lock.native_window;
        let mut frame = window.frame();
        let content_layout_rect = window.contentLayoutRect();
        let style_mask = window.styleMask();
        if !style_mask.contains(NSWindowStyleMask::FullSizeContentView) {
            frame.origin.y -= frame.size.height - content_layout_rect.size.height;
        }
        frame
    }

    fn screen_point_to_gpui_point(&self, position: NSPoint) -> Point<Pixels> {
        let frame = self.get_window_frame();
        let window_x = position.x - frame.origin.x;
        let window_y = frame.size.height - (position.y - frame.origin.y);

        point(px(window_x as f32), px(window_y as f32))
    }

    // Things to test if you're modifying this method:
    //  U.S. layout:
    //   - The IME consumes characters like 'j' and 'k', which makes paging through `less` in
    //     the terminal behave incorrectly by default. This behavior should be patched by our
    //     IME integration
    //   - `alt-t` should open the tasks menu
    //   - In vim mode, this keybinding should work:
    //     ```
    //        {
    //          "context": "Editor && vim_mode == insert",
    //          "bindings": {"j j": "vim::NormalBefore"}
    //        }
    //     ```
    //     and typing 'j k' in insert mode with this keybinding should insert the two characters
    //  Brazilian layout:
    //   - `" space` should create an unmarked quote
    //   - `" backspace` should delete the marked quote
    //   - `" "`should create an unmarked quote and a second marked quote
    //   - `" up` should insert a quote, unmark it, and move up one line
    //   - `" cmd-down` should insert a quote, unmark it, and move to the end of the file
    //   - `cmd-ctrl-space` and clicking on an emoji should type it
    //  Czech (QWERTY) layout:
    //   - in vim mode `option-4`  should go to end of line (same as $)
    //  Japanese (Romaji) layout:
    //   - type `a i left down up enter enter` should create an unmarked text "愛"
    fn handle_key_event(&self, native_event: &NSEvent, key_equivalent: bool) -> bool {
        let window_state = self.window_state();
        let mut lock = window_state.as_ref().lock();

        let window_height = lock.content_size().height;
        let event = unsafe { PlatformInput::from_native(native_event, Some(window_height)) };

        let Some(event) = event else {
            return false;
        };

        let run_callback = |event: PlatformInput| -> bool {
            let mut callback = window_state.as_ref().lock().event_callback.take();
            let handled = if let Some(callback) = callback.as_mut() {
                !callback(event).propagate
            } else {
                false
            };
            window_state.as_ref().lock().event_callback = callback;
            handled
        };

        match event {
            PlatformInput::KeyDown(mut key_down_event) => {
                // For certain keystrokes, macOS will first dispatch a "key equivalent" event.
                // If that event isn't handled, it will then dispatch a "key down" event. GPUI
                // makes no distinction between these two types of events, so we need to ignore
                // the "key down" event if we've already just processed its "key equivalent" version.
                if key_equivalent {
                    lock.last_key_equivalent = Some(key_down_event.clone());
                } else if lock.last_key_equivalent.take().as_ref() == Some(&key_down_event) {
                    return false;
                }

                drop(lock);

                let is_composing = self
                    .with_input_handler(|input_handler| input_handler.marked_text_range())
                    .flatten()
                    .is_some();

                // If we're composing, send the key to the input handler first;
                // otherwise we only send to the input handler if we don't have a matching binding.
                // The input handler may call `do_command_by_selector` if it doesn't know how to handle
                // a key. If it does so, it will return YES so we won't send the key twice.
                // We also do this for non-printing keys (like arrow keys and escape) as the IME menu
                // may need them even if there is no marked text;
                // however we skip keys with control or the input handler adds control-characters to the buffer.
                // and keys with function, as the input handler swallows them.
                if is_composing
                    || (key_down_event.keystroke.key_char.is_none()
                        && !key_down_event.keystroke.modifiers.control
                        && !key_down_event.keystroke.modifiers.function)
                {
                    {
                        let mut lock = window_state.as_ref().lock();
                        lock.keystroke_for_do_command = Some(key_down_event.keystroke.clone());
                        lock.do_command_handled.take();
                        drop(lock);
                    }

                    let handled: bool = self
                        .inputContext()
                        .map(|ctx| ctx.handleEvent(native_event))
                        .unwrap_or(false);
                    window_state.as_ref().lock().keystroke_for_do_command.take();
                    if let Some(handled) = window_state.as_ref().lock().do_command_handled.take() {
                        return handled;
                    } else if handled {
                        return true;
                    }

                    let handled = run_callback(PlatformInput::KeyDown(key_down_event));
                    return handled;
                }

                let handled = run_callback(PlatformInput::KeyDown(key_down_event.clone()));
                if handled {
                    return true;
                }

                if key_down_event.is_held
                    && let Some(key_char) = key_down_event.keystroke.key_char.as_ref()
                {
                    let handled = self.with_input_handler(|input_handler| {
                        if !input_handler.apple_press_and_hold_enabled() {
                            input_handler.replace_text_in_range(None, key_char);
                            return true;
                        }
                        false
                    });
                    if handled == Some(true) {
                        return true;
                    }
                }

                // Don't send key equivalents to the input handler if there are key modifiers other
                // than Function key, or macOS shortcuts like cmd-` will stop working.
                if key_equivalent && key_down_event.keystroke.modifiers != Modifiers::function() {
                    return false;
                }

                if let Some(input_context) = self.inputContext() {
                    input_context.handleEvent(native_event)
                } else {
                    false
                }
            }

            PlatformInput::KeyUp(_) => {
                drop(lock);
                run_callback(event)
            }

            _ => false,
        }
    }

    fn handle_view_event(&self, native_event: &NSEvent) {
        let window_state = self.window_state();
        let weak_window_state = Arc::downgrade(&window_state);
        let mut lock = window_state.as_ref().lock();
        let window_height = lock.content_size().height;
        let event = unsafe { PlatformInput::from_native(native_event, Some(window_height)) };

        if let Some(mut event) = event {
            match &mut event {
                PlatformInput::MouseDown(
                    event @ MouseDownEvent {
                        button: MouseButton::Left,
                        modifiers: Modifiers { control: true, .. },
                        ..
                    },
                ) => {
                    // On mac, a ctrl-left click should be handled as a right click.
                    *event = MouseDownEvent {
                        button: MouseButton::Right,
                        modifiers: Modifiers {
                            control: false,
                            ..event.modifiers
                        },
                        click_count: 1,
                        ..*event
                    };
                }

                // Handles focusing click.
                PlatformInput::MouseDown(
                    event @ MouseDownEvent {
                        button: MouseButton::Left,
                        ..
                    },
                ) if (lock.first_mouse) => {
                    *event = MouseDownEvent {
                        first_mouse: true,
                        ..*event
                    };
                    lock.first_mouse = false;
                }

                // Because we map a ctrl-left_down to a right_down -> right_up let's ignore
                // the ctrl-left_up to avoid having a mismatch in button down/up events if the
                // user is still holding ctrl when releasing the left mouse button
                PlatformInput::MouseUp(
                    event @ MouseUpEvent {
                        button: MouseButton::Left,
                        modifiers: Modifiers { control: true, .. },
                        ..
                    },
                ) => {
                    *event = MouseUpEvent {
                        button: MouseButton::Right,
                        modifiers: Modifiers {
                            control: false,
                            ..event.modifiers
                        },
                        click_count: 1,
                        ..*event
                    };
                }

                _ => {}
            };

            match &event {
                PlatformInput::MouseDown(_) => {
                    drop(lock);
                    self.inputContext().map(|ctx| ctx.handleEvent(native_event));
                    lock = window_state.as_ref().lock();
                }
                PlatformInput::MouseMove(
                    event @ MouseMoveEvent {
                        pressed_button: Some(_),
                        ..
                    },
                ) => {
                    // Synthetic drag is used for selecting long buffer contents while buffer is being scrolled.
                    // External file drag and drop is able to emit its own synthetic mouse events which will conflict
                    // with these ones.
                    if !lock.external_files_dragged {
                        lock.synthetic_drag_counter += 1;
                        let executor = lock.executor.clone();
                        executor
                            .spawn(synthetic_drag(
                                weak_window_state,
                                lock.synthetic_drag_counter,
                                event.clone(),
                            ))
                            .detach();
                    }
                }

                PlatformInput::MouseUp(MouseUpEvent { .. }) => {
                    lock.synthetic_drag_counter += 1;
                }

                PlatformInput::ModifiersChanged(ModifiersChangedEvent { modifiers, capslock }) => {
                    // Only raise modifiers changed event when they have actually changed
                    if let Some(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                        modifiers: prev_modifiers,
                        capslock: prev_capslock,
                    })) = &lock.previous_modifiers_changed_event
                        && prev_modifiers == modifiers
                        && prev_capslock == capslock
                    {
                        return;
                    }

                    lock.previous_modifiers_changed_event = Some(event.clone());
                }

                _ => {}
            }

            if let Some(mut callback) = lock.event_callback.take() {
                drop(lock);
                callback(event);
                window_state.lock().event_callback = Some(callback);
            }
        }
    }

    fn with_input_handler<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut PlatformInputHandler) -> R,
    {
        let window_state = self.window_state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut input_handler) = lock.input_handler.take() {
            drop(lock);
            let result = f(&mut input_handler);
            window_state.lock().input_handler = Some(input_handler);
            Some(result)
        } else {
            None
        }
    }
}

fn nsrange_is_valid(range: &NSRange) -> bool {
    range.location != NSNotFound as usize
}

fn nsrange_to_range(range: &NSRange) -> Option<Range<usize>> {
    if nsrange_is_valid(range) {
        let start = range.location;
        let end = start + range.length;
        Some(start..end)
    } else {
        None
    }
}

fn maybe_attributed_string(text: &AnyObject) -> Retained<NSString> {
    if let Some(text) = text.downcast_ref::<NSAttributedString>() {
        text.string()
    } else if let Some(text) = text.downcast_ref::<NSString>() {
        unsafe { Retained::retain(text as *const NSString as *mut NSString).unwrap() }
    } else {
        panic!("Expected an NSAttributedString or NSString");
    }
}
