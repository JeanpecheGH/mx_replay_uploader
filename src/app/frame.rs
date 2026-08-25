use crate::app::{ClosingState, ConnectPromise, ReplayUploaderApp, UploadPromise};
use crate::pref;
use eframe::Frame;
use eframe::glow::Context;

#[derive(Clone)]
pub enum State {
    Credentials(bool, Option<String>),
    Connecting(ConnectPromise),
    ReplayFolder(bool, Option<String>),
    ListView,
    LogView,
    Uploading(UploadPromise),
}

impl eframe::App for ReplayUploaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // First, we read incoming events from the watcher
        self.receive_event();

        let state = self.state.clone();
        match &state {
            State::Credentials(forced, error) => {
                self.login_form(ctx, *forced, error.as_ref());
            }
            State::Connecting(promise) => self.wait_login(ctx, promise),
            State::ReplayFolder(forced, error) => {
                self.folder_form(ctx, *forced, error.as_ref());
            }
            State::ListView => {
                self.list_view(ctx);
            }
            State::LogView => {
                self.log_view(ctx);
            }
            State::Uploading(promise) => {
                self.wait_upload(ctx, promise);
            }
        }

        // Cancel the close
        if ctx.input(|i| i.viewport().close_requested()) {
            match self.closing_state {
                ClosingState::Open => {
                    if self.upload_replays() {
                        self.closing_state = ClosingState::Uploading;
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    }
                }
                ClosingState::Uploading => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
                ClosingState::Closing => (),
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        match pref::save_pref(&self.pref) {
            Ok(()) => println!("Preferences successfully saved"),
            Err(e) => println!("Error saving pref: {}", e),
        }
    }
}
