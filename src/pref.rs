extern crate preferences;
use preferences::{AppInfo, Preferences, PreferencesMap};

const APP_INFO: AppInfo = AppInfo {
    name: "MxReplayUploader",
    author: "Jeanpeche",
};
const PREFS_KEY: &str = "conf";

#[derive(Default)]
pub struct Pref {
    pub autosave_path: String,
    pub username: String,
    pub password: String,
}

pub fn load_pref() -> Result<Pref, preferences::PreferencesError> {
    let pref_map = match PreferencesMap::<String>::load(&APP_INFO, PREFS_KEY) {
        Ok(pref_map) => pref_map,
        Err(e) => {
            println!(
                "Unable to load existing preferences, initialize preferences : {}",
                e
            );
            save_pref(&Pref::default())?;
            PreferencesMap::<String>::load(&APP_INFO, PREFS_KEY)?
        }
    };
    Ok(Pref {
        autosave_path: pref_map
            .get("autosave_path")
            .map_or("", |s| &s[..])
            .to_string(),
        username: pref_map.get("username").map_or("", |s| &s[..]).to_string(),
        password: pref_map.get("password").map_or("", |s| &s[..]).to_string(),
    })
}

pub fn save_pref(prefs: &Pref) -> Result<(), preferences::PreferencesError> {
    let mut pref_map: PreferencesMap<String> = PreferencesMap::new();
    pref_map.insert("autosave_path".into(), prefs.autosave_path.clone());
    pref_map.insert("username".into(), prefs.username.clone());
    pref_map.insert("password".into(), prefs.password.clone());
    pref_map.save(&APP_INFO, PREFS_KEY)?;
    Ok(())
}
