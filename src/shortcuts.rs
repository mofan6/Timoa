//! Local focus-page shortcuts. Physical keys work with any input language.
use crate::{Controller, MainWindow, native};
use slint::{ComponentHandle, winit_030::{WinitWindowAccessor, EventResult, winit::{
    event::{ElementState, TouchPhase, WindowEvent}, keyboard::{KeyCode, ModifiersState, PhysicalKey},
}}};
use std::{rc::Rc, time::{Duration, Instant}};

#[derive(Default)]
struct Sequence { first: Option<Instant>, count: u8, page: Option<i32> }
impl Sequence {
    fn reset(&mut self) { self.first = None; self.count = 0; self.page = None; }
    fn press_m(&mut self, page: i32) -> bool {
        let now = Instant::now();
        if self.page != Some(page) || self.first.is_none_or(|first| now.duration_since(first) > Duration::from_secs(1)) {
            self.reset(); self.first = Some(now); self.page = Some(page);
        }
        self.count += 1;
        if self.count == 3 { self.reset(); true } else { false }
    }
}

fn focus_action(controller: &Controller, start: bool) {
    let Ok(mut app) = controller.try_borrow_mut() else { return; };
    let Some(ui) = app.ui.upgrade() else { return; };
    if ui.get_page() == 4 && app.engine.doc.settings.stopwatch_enabled {
        // Starting is idempotent; N never resumes an already paused stopwatch.
        if app.engine.doc.stopwatch.running == start { return; }
        app.engine.stopwatch_action("toggle");
    } else if ui.get_page() == 0 {
        let timer = app.engine.focus_timer().map(|timer| (timer.id.clone(), timer.finished, timer.paused));
        let Some((id, finished, paused)) = timer else {
            if start { app.start_countdown(); }
            return;
        };
        if !start && app.engine.cycle_remaining(&id).is_some() {
            app.engine.timer_action(&id, "toggle");
        } else if start && finished {
            app.engine.timer_action(&id, "reset");
            if paused { app.engine.timer_action(&id, "toggle"); }
        } else if !finished && (start && paused || !start && !paused) {
            app.engine.timer_action(&id, "toggle");
        } else { return; }
    } else { return; }
    app.commit(); app.refresh();
}

pub fn install(controller: &Controller, ui: &MainWindow) {
    let weak = Rc::downgrade(controller);
    let input_ui = ui.as_weak();
    let mut sequence = Sequence::default();
    let mut modifiers = ModifiersState::empty();
    // Filter this main window only. No global keyboard hook or extra timer.
    ui.window().on_winit_window_event(move |_, event| {
        match event {
            WindowEvent::Focused(false) => {
                sequence.reset(); modifiers = ModifiersState::empty();
                if let Some(ui) = input_ui.upgrade() { ui.invoke_dismiss_name_history(); }
            }
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                if let Some(ui) = input_ui.upgrade() { ui.invoke_dismiss_name_history(); }
            }
            WindowEvent::ModifiersChanged(value) => { modifiers = value.state(); sequence.reset(); }
            WindowEvent::MouseInput { state: ElementState::Pressed, .. } => {
                sequence.reset();
                if let Some(ui) = input_ui.upgrade() {
                    if let Some(point) = native::pointer_position(ui.window()) {
                        ui.invoke_name_history_pointer(point.x, point.y);
                    }
                }
            }
            WindowEvent::MouseWheel { .. } => {
                if let Some(ui) = input_ui.upgrade() {
                    if let Some(point) = native::pointer_position(ui.window()) {
                        ui.invoke_name_history_scroll(point.x, point.y);
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                sequence.reset();
                if touch.phase == TouchPhase::Started {
                    if let Some(ui) = input_ui.upgrade() {
                        let scale = ui.window().scale_factor();
                        ui.invoke_name_history_pointer(touch.location.x as f32 / scale, touch.location.y as f32 / scale);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                if event.state == ElementState::Pressed && event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    if let Some(ui) = input_ui.upgrade() { ui.invoke_dismiss_name_history(); }
                }
                let Some(controller) = weak.upgrade() else { return EventResult::Propagate; };
                let Ok(app) = controller.try_borrow() else { sequence.reset(); return EventResult::Propagate; };
                let Some(ui) = app.ui.upgrade() else { return EventResult::Propagate; };
                let page = ui.get_page();
                let enabled = !app.compact && app.confirm.is_none() && app.about.is_none() && app.alert.is_none()
                    && !ui.get_focus_shortcuts_editing()
                    && (page == 0 || page == 4 && app.engine.doc.settings.stopwatch_enabled)
                    && !modifiers.control_key() && !modifiers.alt_key() && !modifiers.super_key();
                drop(app);
                if !enabled || *is_synthetic { sequence.reset(); return EventResult::Propagate; }
                if event.state != ElementState::Pressed { return EventResult::Propagate; }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyM) => {
                        // OS auto-repeat is not a new tap: hold M cannot start a round.
                        if !event.repeat && sequence.press_m(page) { focus_action(&controller, true); }
                        return EventResult::PreventDefault;
                    }
                    PhysicalKey::Code(KeyCode::KeyN) => {
                        sequence.reset();
                        if !event.repeat { focus_action(&controller, false); }
                        return EventResult::PreventDefault;
                    }
                    _ => sequence.reset(),
                }
            }
            _ => {}
        }
        EventResult::Propagate
    });
}
