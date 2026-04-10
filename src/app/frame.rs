use crate::app::{ConnectPromise, ReplayUploaderApp, UploadPromise};
use crate::pref;
use eframe::Frame;
use eframe::glow::Context;

#[derive(Clone)]
pub enum State {
    Credentials(bool, Option<String>),
    Connecting(ConnectPromise),
    ReplayFolder(bool, Option<String>),
    ListView,
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
            State::Uploading(promise) => {
                self.wait_upload(ctx, promise);
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        self.upload_replays();
        match pref::save_pref(&self.pref) {
            Ok(()) => println!("Preferences successfully saved"),
            Err(e) => println!("Error saving pref: {}", e),
        }
    }
}
