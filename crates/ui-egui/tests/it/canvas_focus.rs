//! The graph canvas as a Tab stop: it keeps the focus Tab gives it, a press and an opening graph give it the focus, Tab
//! over it opens the node search, and its keys work as with nothing focused

use egui::{Event, Id, Key, Modifiers, PointerButton, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use materialcraft_ui_egui::framework::{canvas_id, install_headless};
use materialcraft_ui_egui::{MaterialCraftApp, UiState};

type AppHarness = Harness<'static, MaterialCraftApp>;

fn harness(session: materialcraft_engine::Session) -> AppHarness {
    let mut h = Harness::builder().with_context_setup(install_headless).with_size(vec2(1600.0, 960.0)).build_ui_state(
        |ui, app: &mut MaterialCraftApp| {
            app.step(&ui.ctx().clone());
            app.draw(ui);
        },
        MaterialCraftApp::headless(session, UiState::default()),
    );
    h.run_steps(8);
    h
}

fn focused(h: &AppHarness) -> Option<Id> {
    h.ctx.memory(|memory| memory.focused())
}

fn press(h: &mut AppHarness, modifiers: Modifiers, key: Key) -> Option<Id> {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
    focused(h)
}

fn search_open(h: &AppHarness) -> bool {
    h.query_by_label("Search nodes").is_some()
}

fn hover_canvas(h: &mut AppHarness) {
    h.hover_at(h.state().graph.view.center());
    h.run_steps(2);
}

#[test]
fn tab_walks_over_the_canvas_to_the_stops_after_it_and_back() {
    let mut h = harness(materialcraft_engine::new_session());
    let canvas = Some(canvas_id());
    assert_eq!(focused(&h), canvas, "an opened graph has the keyboard");

    let after = press(&mut h, Modifiers::NONE, Key::Tab);
    assert!(after.is_some() && after != canvas, "Tab moves on from the canvas");
    assert_eq!(press(&mut h, Modifiers::SHIFT, Key::Tab), canvas);
    let before = press(&mut h, Modifiers::SHIFT, Key::Tab);
    assert!(before.is_some() && before != canvas && before != after, "Shift+Tab leaves it backward");
    assert_eq!(press(&mut h, Modifiers::NONE, Key::Tab), canvas, "Tab from the stop before it stops at it");
    assert_eq!(press(&mut h, Modifiers::NONE, Key::Tab), after, "and reaches the stop after it");
    assert!(!search_open(&h));
}

#[test]
fn tab_with_the_pointer_over_the_canvas_opens_the_search_and_shift_tab_and_f6_leave_it() {
    let mut h = harness(materialcraft_engine::new_session());
    let canvas = Some(canvas_id());
    hover_canvas(&mut h);
    for (modifiers, key) in [(Modifiers::SHIFT, Key::Tab), (Modifiers::NONE, Key::F6)] {
        let left = press(&mut h, modifiers, key);
        assert!(left.is_some() && left != canvas, "{key:?} leaves the canvas");
        assert!(!search_open(&h));
        h.ctx.memory_mut(|memory| memory.surrender_focus(left.unwrap()));
        h.run_steps(2);
        assert_eq!(focused(&h), canvas, "with nothing focused, the canvas under the pointer takes the keyboard");
    }

    press(&mut h, Modifiers::NONE, Key::Tab);
    assert!(search_open(&h), "Tab opens the search");
    assert_eq!(press(&mut h, Modifiers::NONE, Key::Escape), canvas, "closing it gives the keyboard back");
    assert!(!search_open(&h));
}

#[test]
fn a_press_in_the_canvas_takes_the_keyboard_from_a_toolbar_button() {
    let mut h = harness(materialcraft_engine::new_session());
    h.get_by_label("Open package... (Ctrl+O)").focus();
    h.run_steps(2);
    press(&mut h, Modifiers::SHIFT, Key::Tab);
    assert!(h.get_by_label("New graph... (Ctrl+N)").is_focused());

    hover_canvas(&mut h);
    assert!(h.get_by_label("New graph... (Ctrl+N)").is_focused(), "the pointer alone takes no focus from a widget");
    let pos = h.state().graph.view.center();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(2);
        assert_eq!(focused(&h), Some(canvas_id()));
    }
    press(&mut h, Modifiers::NONE, Key::Space);
    assert!(search_open(&h), "Space goes to the canvas");
    assert!(!h.state().ui.windows.new_graph, "and not to the button");
}

#[test]
fn shortcuts_run_with_the_canvas_focused() {
    let mut h = harness(materialcraft_engine::sample_session());
    assert_eq!(focused(&h), Some(canvas_id()));
    let nodes = |h: &AppHarness| h.state().session.graph().unwrap().nodes.len();
    let before = nodes(&h);

    press(&mut h, Modifiers::COMMAND, Key::A);
    assert_eq!(h.state().session.selection.len(), before, "Ctrl+A selects every node");
    press(&mut h, Modifiers::NONE, Key::Delete);
    assert_eq!(nodes(&h), 0, "Delete deletes them");
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(nodes(&h), before, "Ctrl+Z brings them back");
    assert_eq!(focused(&h), Some(canvas_id()));
}
