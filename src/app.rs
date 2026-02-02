mod frame;

use crate::app::frame::State;
use crate::gbx_parser::GbxHeader;
use crate::mx_client::{ClientError, MxClient};
use crate::pref::Pref;
use crate::watcher;
use eframe::emath::Align;
use egui::{Layout, RichText};
use egui_extras::{Column, TableBody, TableBuilder};
use poll_promise::Promise;
use rfd::FileDialog;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

// We should have 1 reader (UI) and 1 writer (Event watcher) at anytime
// Mutex is fine for this usage
pub type ReplayMap = Arc<Mutex<HashMap<String, (GbxHeader, usize)>>>;
pub type ConnectPromise = Rc<Promise<Result<(), ClientError>>>;
pub type UploadPromise = Rc<Promise<Result<(), ClientError>>>;

pub struct ReplayUploaderApp {
    replays: ReplayMap,
    username: String,
    password: String,
    client: Arc<MxClient>,
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

    pub fn watch_folder(&self) {
        let path: String = self.pref.autosave_path.clone().unwrap();
        let replays: ReplayMap = self.replays.clone();
        let _ = thread::spawn(move || {
            let _ = watcher::watch_folder(&path, replays);
        });
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
        let client: Arc<MxClient> = self.client.clone();
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
            let client: Arc<MxClient> = self.client.clone();
            let promise: Promise<Result<(), ClientError>> = Promise::spawn_async(async move {
                for gbx in r {
                    if let Some(id) = client.get_map_id(gbx.uid()).await.unwrap() {
                        println!("Uploading replay {} {} {}", gbx.name(), gbx.uid(), id);
                        match client.upload_replay(&gbx.path, id).await {
                            Ok(_) => println!("Upload sucessful!"),
                            Err(e) => println!("Upload failed {:?}", e),
                        }
                    } else {
                        println!("Map {} does not exist on Mania Exchange", gbx.name())
                    }
                }
                Ok(())
            });
            replays.clear();
            self.state = State::Uploading(Rc::new(promise));
        }
    }

    pub fn with_pref(pref: Pref) -> Self {
        let client = MxClient::build_mx_client().unwrap();
        Self {
            replays: Arc::new(Mutex::new(HashMap::new())),
            username: pref.username.clone().unwrap_or_default(),
            password: pref.password.clone().unwrap_or_default(),
            client: Arc::new(client),
            pref,
            state: State::Credentials(false, None),
        }
    }

    /// !!!!!!!!!!!!!!!!! ///
    /// Display functions ///
    /// !!!!!!!!!!!!!!!!! ///

    fn wait_login_spinner(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Connecting to Mania Exchange");
            ui.spinner();
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
            self.wait_login_spinner(ctx);
        }
    }

    fn wait_login(&mut self, ctx: &egui::Context, promise: &ConnectPromise) {
        if let Some(result) = promise.ready() {
            match result {
                Ok(()) => {
                    self.state = State::ReplayFolder(false, None);
                }
                Err(ClientError::Error(s)) => {
                    println!("{:?}", s);
                    self.state = State::Credentials(
                        false,
                        Some("Error connecting to Mania Exchange".to_string()),
                    );
                }
            }
        }
        // Draw waiting panel in all cases
        self.wait_login_spinner(ctx);
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

    fn populate_table(&mut self, body: &mut TableBody) {
        let replays = self.replays.lock().expect("Locking replays failed from UI");
        let mut headers: Vec<(GbxHeader, usize)> = replays.values().cloned().collect();
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
    }

    fn list_view(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right panel")
            .exact_width(200.0)
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
            ui.vertical(|ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::remainder().at_least(200.0))
                    .column(Column::auto().at_least(200.0))
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
                        self.populate_table(&mut body);
                    });
            });
        });
    }

    fn wait_upload(&mut self, ctx: &egui::Context, promise: &UploadPromise) {
        if let Some(result) = promise.ready() {
            match result {
                Ok(()) => {
                    self.state = State::ListView;
                }
                Err(error) => {
                    println!("{error:?}");
                    self.state = State::ListView;
                }
            }
        }
        // Draw waiting panel in all cases
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Uploading replays to Mania Exchange");
            ui.spinner();
        });
    }
}
