mod frame;

use crate::app::frame::State;
use crate::gbx_parser::{GbxHeader, parse_from_file};
use crate::logs::{Log, LogKind};
use crate::mx_client::MxClient;
use crate::mx_client::client_error::ClientError;
use crate::mx_client::upload_response::UploadResponse;
use crate::pref::Pref;
use crate::watcher;
use eframe::emath::Align;
use egui::{Layout, RichText, Spinner};
use egui_extras::{Column, TableBuilder};
use notify::event::ModifyKind::Name;
use notify::{Event, EventKind};
use poll_promise::Promise;
use rfd::FileDialog;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

// We should have 1 reader (UI) and 1 writer (Event watcher) at anytime
// Mutex is fine for this usage
pub type ReplayMap = Mutex<HashMap<String, (GbxHeader, usize)>>;
pub type ConnectPromise = Rc<Promise<Result<(), ClientError>>>;
pub type UploadPromise = Rc<Promise<Vec<(Result<UploadResponse, ClientError>, GbxHeader)>>>;

pub struct ReplayUploaderApp {
    replays: ReplayMap,
    username: String,
    password: String,
    client: MxClient,
    sender: Sender<notify::Result<Event>>,
    receiver: Receiver<notify::Result<Event>>,
    logs: Vec<Log>,
    pref: Pref,
    state: State,
}

impl ReplayUploaderApp {
    fn get_file_dialog() -> FileDialog {
        let path = std::env::current_dir()
            .ok()
            .and_then(|d| d.to_str().map(|s| s.to_string()));
        match path {
            Some(p) => FileDialog::new().set_directory(&p),
            None => FileDialog::new(),
        }
    }

    pub fn watch_folder(&mut self) {
        let path: String = self.pref.autosave_path.clone().unwrap();
        let sender = self.sender.clone();

        self.log(
            LogKind::Watcher,
            format!("Watching {} for new replays...", path),
        );
        //This thread loops indefinitely
        let _ = thread::spawn(move || {
            let _ = watcher::watch_folder(&path, sender);
        });
    }

    fn receive_event(&mut self) {
        // Don't handle receiving errors for now
        while let Ok(res) = self.receiver.try_recv() {
            match res {
                Ok(event) => {
                    if let EventKind::Modify(Name(notify::event::RenameMode::To)) = event.kind
                        && let Some(p) = event.paths.first()
                        && p.to_str().unwrap().ends_with(".Gbx")
                    {
                        match parse_from_file(p.to_str().unwrap()) {
                            Ok(hdr) => {
                                self.log(
                                    LogKind::Record,
                                    format!("\"{}\" : {}", hdr.name(), hdr.time()),
                                );
                                let mut r = self
                                    .replays
                                    .lock()
                                    .expect("Locking replays failed from watcher");
                                let seconds: usize = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    as usize;
                                let _ = r.insert(String::from(hdr.uid()), (hdr, seconds));
                            }
                            Err(e) => self.log(LogKind::Parser, format!("Error: {:?}", e)),
                        }
                    }
                }
                Err(e) => self.log(LogKind::Watcher, format!("Error: {:?}", e)),
            }
        }
    }

    fn pick_game_folder(&mut self) {
        if let Some(path) = Self::get_file_dialog().pick_folder() {
            let autosave_path = path.display().to_string();
            self.pref.autosave_path = Some(autosave_path.clone());
            self.watch_folder();
            self.state = State::ListView;
        }
    }

    fn set_credentials(&mut self) {
        self.pref.username = Some(self.username.clone());
        self.pref.password = Some(self.password.clone());
    }

    fn connect(&mut self) {
        let username: String = self.username.clone();
        let password: String = self.password.clone();
        let client: MxClient = self.client.clone();
        let promise =
            Promise::spawn_async(async move { client.connect(&username, &password).await });
        self.state = State::Connecting(Rc::new(promise));
    }

    fn upload_replays(&mut self) {
        let mut replays = self
            .replays
            .lock()
            .expect("Locking replays failed from upload");
        if !replays.is_empty() {
            let r: Vec<GbxHeader> = replays.values().map(|(h, _)| h).cloned().collect();
            let client: MxClient = self.client.clone();
            let promise: Promise<Vec<(Result<UploadResponse, ClientError>, GbxHeader)>> =
                Promise::spawn_async(async move { client.upload_all(r).await });
            replays.clear();
            self.state = State::Uploading(Rc::new(promise));
        }
    }

    fn log(&mut self, kind: LogKind, value: String) {
        let log = Log { kind, value };
        println!("{log}");
        self.logs.push(log);
    }

    pub fn with_pref(pref: Pref) -> Self {
        let client: MxClient = MxClient::build_mx_client().unwrap();
        let (sender, receiver) = mpsc::channel::<notify::Result<Event>>();
        Self {
            replays: Mutex::new(HashMap::new()),
            username: pref.username.clone().unwrap_or_default(),
            password: pref.password.clone().unwrap_or_default(),
            client,
            sender,
            receiver,
            logs: Vec::new(),
            pref,
            state: State::Credentials(false, None),
        }
    }

    /// !!!!!!!!!!!!!!!!! ///
    /// Display functions ///
    /// !!!!!!!!!!!!!!!!! ///
    fn wait_spinner(&mut self, ctx: &egui::Context, text: &str) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let height: f32 = ui.max_rect().height();
                ui.add_space(height / 5.0);
                ui.label(RichText::from(text).heading().strong());
                ui.add_space(height / 5.0);
                ui.add(Spinner::new().size(60.0))
            });
        });
    }

    fn login_form(&mut self, ctx: &egui::Context, forced: bool, error: Option<&String>) {
        if forced || error.is_some() || self.pref.username.is_none() || self.pref.password.is_none()
        {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("");
                    ui.label("Please set your ManiaExchange username and password.");
                    ui.label("");
                    ui.add(egui::TextEdit::singleline(&mut self.username).hint_text("Username"));
                    ui.label("");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .hint_text("Password")
                            .password(true),
                    );
                    ui.label("");
                    if ui.button("Set credentials").clicked() {
                        self.set_credentials();
                        self.connect();
                    }
                });
            });
        } else {
            self.connect();
            // We have to draw something every frame,
            // Start the spinner now
            self.wait_spinner(ctx, "Connecting to ManiaExchange");
        }
    }

    fn wait_login(&mut self, ctx: &egui::Context, promise: &ConnectPromise) {
        if let Some(result) = promise.ready() {
            match result {
                Ok(()) => {
                    self.state = State::ReplayFolder(false, None);
                    self.log(LogKind::Client, "Connected to ManiaExchange".to_string());
                }
                Err(e) => {
                    self.state = State::Credentials(
                        false,
                        Some("Error connecting to ManiaExchange".to_string()),
                    );
                    self.log(
                        LogKind::Client,
                        format!("Error connecting to ManiaExchange: {e}"),
                    );
                }
            }
        }
        // Draw waiting panel in all cases
        self.wait_spinner(ctx, "Connecting to ManiaExchange");
    }

    fn folder_form(&mut self, ctx: &egui::Context, forced: bool, error: Option<&String>) {
        if forced || error.is_some() || self.pref.autosave_path.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("");
                    ui.label("The autosave path is not defined.");
                    ui.label("");
                    ui.label("Please select the folder where your replays are saved.");
                    ui.label("");
                    ui.label("Ex: \"../ManiaPlanet/DonneesPerso/Replays/Autosaves\"");
                    ui.label("");
                    if ui.button("Select autosave folder").clicked() {
                        self.pick_game_folder();
                    }
                });
            });
        } else {
            self.watch_folder();
            self.state = State::ListView;
            // Already show list view in advance
            self.list_view(ctx);
        }
    }

    fn list_view(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right panel")
            .exact_width(150.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.label("");
                    ui.label("");
                    if ui
                        .button(RichText::from("Upload all").heading().strong())
                        .clicked()
                    {
                        self.upload_replays();
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let replays = self.replays.lock().expect("Locking replays failed from UI");
            if replays.is_empty() {
                // Waiting message
                ui.vertical_centered(|ui| {
                    let height: f32 = ui.max_rect().height();
                    ui.add_space(height / 4.0);
                    ui.label(RichText::from("Waiting for new replays").heading().strong());
                    ui.add_space(height / 6.0);
                    let folder: String = format!(
                        "Watching \"{}\"",
                        self.pref
                            .autosave_path
                            .clone()
                            .unwrap_or("Not set".to_string())
                    );
                    ui.label(folder);
                });
            } else {
                ui.vertical(|ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::remainder().at_least(200.0))
                        .column(Column::auto().at_least(150.0))
                        .column(Column::exact(150.0))
                        //Without this, end of line start disappearing to right when resizing (default top_down is wrong)
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .header(30.0, |mut header| {
                            header.col(|ui| {
                                ui.label(RichText::from("Map Name").heading().strong());
                            });
                            header.col(|ui| {
                                ui.label(RichText::from("Author").heading().strong());
                            });
                            header.col(|ui| {
                                ui.label(RichText::from("Time").heading().strong());
                            });
                        })
                        .body(|mut body| {
                            let mut headers: Vec<(GbxHeader, usize)> =
                                replays.values().cloned().collect();
                            headers.sort_by(|(_, a), (_, b)| a.cmp(b));
                            for (hdr, _) in headers {
                                body.row(30.0, |mut row| {
                                    row.col(|ui| {
                                        ui.horizontal_centered(|ui| {
                                            ui.label(hdr.name());
                                        });
                                    });
                                    row.col(|ui| {
                                        ui.horizontal_centered(|ui| {
                                            ui.label(hdr.author());
                                        });
                                    });
                                    row.col(|ui| {
                                        ui.horizontal_centered(|ui| {
                                            ui.label(hdr.time());
                                        });
                                    });
                                });
                            }
                        });
                });
            }
        });
    }

    fn wait_upload(&mut self, ctx: &egui::Context, promise: &UploadPromise) {
        if let Some(result) = promise.ready() {
            for (r, hdr) in result {
                match r {
                    Ok(resp) => {
                        self.log(
                            LogKind::Upload,
                            format!(
                                "Replay on \"{}\" uploaded, new rank {}",
                                hdr.name(),
                                resp.position()
                            ),
                        );
                    }
                    Err(error) => {
                        self.log(
                            LogKind::Upload,
                            format!("Replay on \"{}\" not uploaded : {error}", hdr.name()),
                        );
                    }
                }
            }
            self.state = State::ListView;
        }
        // Draw waiting panel in all cases
        self.wait_spinner(ctx, "Uploading replays to ManiaExchange")
    }
}
