extern crate preferences;
use preferences::{AppInfo, Preferences, PreferencesMap};

const APP_INFO: AppInfo = AppInfo {
    name: "MxReplayUploader",
    author: "Jeanpeche",
};
const PREFS_KEY: &str = "conf";

#[derive(Default)]
pub struct Pref {
    pub autosave_path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
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
        autosave_path: pref_map.get("autosave_path").cloned(),
        username: pref_map.get("username").cloned(),
        password: pref_map.get("password").cloned(),
    })
}

pub fn save_pref(prefs: &Pref) -> Result<(), preferences::PreferencesError> {
    let mut pref_map: PreferencesMap<String> = PreferencesMap::new();
    prefs.autosave_path.iter().for_each(|path| {
        let _ = pref_map.insert("autosave_path".into(), path.clone());
    });
    prefs.username.iter().for_each(|username| {
        let _ = pref_map.insert("username".into(), username.clone());
    });
    prefs.password.iter().for_each(|password| {
        let _ = pref_map.insert("password".into(), password.clone());
    });
    pref_map.save(&APP_INFO, PREFS_KEY)?;
    Ok(())
}
