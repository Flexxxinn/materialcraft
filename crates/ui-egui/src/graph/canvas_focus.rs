//! The graph editor's canvas widget and its keyboard focus

use crate::MaterialCraftApp;
use crate::framework::{canvas_id, focus_canvas, focused_widget, host_canvas, overlay_open};
use crate::shell::modality::Modality;
use egui::{Id, Rect, Response, Sense, Ui, Vec2};

/// Allocate and draw the canvas widget: every editor (material, function, FX-Map) draws the one that has the focus
pub(crate) fn allocate(app: &MaterialCraftApp, ui: &mut Ui, size: Vec2) -> (Rect, Response) {
    let (_, rect) = ui.allocate_space(size);
    let response = ui.interact(rect, canvas_id(), Sense::click_and_drag());
    // Declared before the widgets on the canvas, for its place in Tab order
    host_canvas(ui.ctx(), false);
    focus_on_press(app, ui, &response);
    (rect, response)
}

/// A press anywhere on the canvas gives it the keyboard focus. Real presses only: with the canvas focused, egui makes
/// Enter and Space clicks of it.
fn focus_on_press(app: &MaterialCraftApp, ui: &Ui, canvas: &Response) {
    let ctx = ui.ctx();
    if !ui.input(|input| input.pointer.any_pressed()) || !ui.rect_contains_pointer(canvas.rect) || Modality::capture(app, ctx).is_blocked() {
        return;
    }

    // A press in the value being edited on a node stays there
    let in_editor = ctx.text_edit_focused() && focused_widget(ctx).and_then(|id| ctx.read_response(id)).is_some_and(|field| field.is_pointer_button_down_on());
    if !in_editor {
        focus_canvas(ctx);
    }
}

/// Whether the pointer is over the canvas: `hovered`, or over the node search `search`, which still covers the canvas in
/// the passes after it closes
pub(crate) fn under_pointer(ui: &Ui, canvas: &Response, hovered: bool, search: Id) -> bool {
    let ctx = ui.ctx();
    hovered
        || ctx.pointer_interact_pos().is_some_and(|pointer| canvas.rect.contains(pointer) && ctx.layer_id_at(pointer).is_some_and(|layer| layer.id == search))
}

/// Settle the focus and Tab once the canvas has handled its input. A graph that opens or becomes active asks for the
/// focus, and so does the canvas while nothing is focused and Tab opens its node search (`opens_search`): a click on a
/// panel leaves the focus nowhere. It waits for a dialog, menu or popup to close, and leaves the focus with a text field
/// and with a widget of another window.
pub(crate) fn settle(app: &mut MaterialCraftApp, ui: &Ui, opens_search: bool) {
    let ctx = ui.ctx();
    let opens_search = opens_search && app.ui.prefs.tab_opens_node_search && !Modality::capture(app, ctx).is_blocked();
    let shown = Some((app.session.document_id, app.session.graph, app.graph.annotation_target.clone()));
    if app.graph.shown != shown {
        app.graph.shown = shown;
        app.graph.wants_focus = true;
    }

    let focused = ctx.memory(|memory| memory.focused());
    let unfocused = opens_search && focused.is_none() && !ui.input(|input| input.pointer.any_down());
    if (app.graph.wants_focus || unfocused) && !overlay_open(app, ctx) {
        app.graph.wants_focus = false;
        let same_window = focused.is_none_or(|id| ctx.read_response(id).is_some_and(|widget| widget.layer_id == ui.layer_id()));
        if same_window && !ctx.text_edit_focused() {
            focus_canvas(ctx);
        }
    }
    host_canvas(ctx, opens_search);
}
