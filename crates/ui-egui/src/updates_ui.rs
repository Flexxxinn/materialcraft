//! Software updates on the desktop: the background check at startup, Help > Check for updates and
//! the prompts that report them, as craft-update's dialogs on the app's
//! [`DialogHost`](craft_ui::dialog::DialogHost)

use craft_ui::dialog::DialogResult;
use craft_update::stack_dialogs::{CheckChoice, UpdateCheckDialog};
use craft_update::updater::{Action, Status, Updater};

use crate::MaterialCraftApp;

pub(crate) struct Runtime {
    updater: Updater,
    /// The desktop app turns the check at start on; tests and scripted runs leave it off
    startup_check: bool,
    check_requested: bool,
    /// The context of the last frame, to wake the UI when a check is queued between frames
    repaint: Option<egui::Context>,
}

impl Default for Runtime {
    fn default() -> Self {
        let updater = Updater::new(&crate::UPDATE_APP, materialcraft_engine::VERSION).with_translator(translate);
        Self { updater, startup_check: false, check_requested: false, repaint: None }
    }
}

/// The updater's words in the interface language (English where the catalog has no row)
pub fn translate(text: &str, args: &[(&str, &str)]) -> String {
    craft_update::translate::fill(crate::i18n::tr(text), args)
}

impl MaterialCraftApp {
    /// Whether the app asks for a newer release when it starts (off until the desktop app turns it on)
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates_rt.startup_check = on;
    }
}

/// Help > Check for updates: the next frame starts a check whose outcome the result dialog shows
pub(crate) fn check(app: &mut MaterialCraftApp) {
    app.updates_rt.check_requested = true;
    if let Some(ctx) = &app.updates_rt.repaint {
        ctx.request_repaint();
    }
}

/// The update prompt and the modal result dialog own the keyboard while they are open: shortcuts
/// must not edit the graph behind them
pub(crate) fn blocks_input(app: &MaterialCraftApp) -> bool {
    let updater = &app.updates_rt.updater;
    holds_keyboard(updater.status(), updater.prompt_open())
}

/// The notice that an update installed leaves the app usable behind it
fn holds_keyboard(status: &Status, prompt_open: bool) -> bool {
    prompt_open || (result_window_shows(status, false) && !matches!(status, Status::Updated(_)))
}

pub(crate) fn tick(app: &mut MaterialCraftApp, ctx: &egui::Context) {
    let rt = &mut app.updates_rt;
    rt.repaint = Some(ctx.clone());
    if std::mem::take(&mut rt.check_requested) {
        rt.updater.check_now();
    } else if rt.startup_check {
        rt.updater.check_at_start();
    }
    rt.updater.tick(ctx);
}

/// Whether the result dialog shows: for every status but idle (a requested check, and the first
/// start after an update, which reports that it installed or why it failed), and never over the
/// startup prompt
fn result_window_shows(status: &Status, prompt_open: bool) -> bool {
    !prompt_open && !matches!(status, Status::Idle)
}

/// One pass of the update flow, before the dialog host runs: answers the result dialog closed in
/// the last pass, drives the startup prompt, and keeps the result dialog open and current while the
/// updater has something to report
pub(crate) fn show(app: &mut MaterialCraftApp, ctx: &egui::Context) {
    // An installation, and the close for a ready update, wait until nothing is unsaved
    let clean = !crate::unsaved::needs_confirmation(&app.session);
    // Answered before the dialog is kept open below, so a dismissed outcome does not open again at once
    match app.framework_dialogs.take::<UpdateCheckDialog>() {
        Some(DialogResult::Ok(CheckChoice::CheckNow | CheckChoice::CheckAgain)) => app.updates_rt.check_requested = true,
        // The next `run` asks the window to close once nothing is unsaved
        Some(DialogResult::Ok(CheckChoice::RestartToUpdate)) => app.updates_rt.updater.restart_to_update(),
        Some(DialogResult::Ok(CheckChoice::Update)) => {
            app.updates_rt.updater.request_install(clean);
        }
        Some(DialogResult::Ok(CheckChoice::Download)) => ctx.open_url(egui::OpenUrl::new_tab(crate::UPDATE_APP.releases_url())),
        // Closing the dialog dismisses the outcome it showed
        Some(_) => app.updates_rt.updater.dismiss_status(),
        None => {}
    }
    match app.updates_rt.updater.run(&mut app.framework_dialogs, clean) {
        Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
        Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        _ => {}
    }
    // A found update opens the startup prompt and leaves the status idle, which ends the result dialog
    let updater = &app.updates_rt.updater;
    let shows = result_window_shows(updater.status(), updater.prompt_open());
    let (status, installing, message) = (updater.status().clone(), updater.is_installing(), updater.status_message());
    let dialogs = &mut app.framework_dialogs;
    if shows && !dialogs.is_open::<UpdateCheckDialog>() {
        dialogs.open(UpdateCheckDialog::new(materialcraft_engine::VERSION));
    } else if !shows && dialogs.is_open::<UpdateCheckDialog>() {
        dialogs.cancel::<UpdateCheckDialog>(ctx);
    }
    if let Some(dialog) = dialogs.get_mut::<UpdateCheckDialog>() {
        dialog.set_status(status);
        dialog.set_installing(installing);
        dialog.set_message(message);
    }
}

/// Opens the update prompt for a test in another module
#[cfg(test)]
pub(crate) fn open_prompt_for_test(app: &mut MaterialCraftApp) {
    tests::open_prompt(app);
}

#[cfg(test)]
#[path = "updates_ui.test.rs"]
mod tests;
