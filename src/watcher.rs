use crate::app::ReplayMap;
use crate::gbx_parser::parse_from_file;
use notify::event::ModifyKind::Name;
use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{path::Path, sync::mpsc};

pub fn watch_folder(path: &str, replays: ReplayMap) -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new(path), RecursiveMode::Recursive)?;

    // Block forever, treat events as they come in
    for res in rx {
        match res {
            Ok(event) => {
                if let EventKind::Modify(Name(notify::event::RenameMode::To)) = event.kind
                    && let Some(p) = event.paths.first()
                    && p.to_str().unwrap().ends_with(".Gbx")
                {
                    match parse_from_file(p.to_str().unwrap()) {
                        Ok(parsed_header) => {
                            println!(
                                "New record on {} : {}",
                                parsed_header.name(),
                                parsed_header.time()
                            );
                            let mut r =
                                replays.lock().expect("Locking replays failed from watcher");
                            let seconds: usize = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                as usize;
                            let _ = r.insert(
                                String::from(parsed_header.uid()),
                                (parsed_header, seconds),
                            );
                        }
                        Err(e) => e.display(),
                    }
                }
            }
            Err(e) => println!("Watcher error: {:?}", e),
        }
    }

    Ok(())
}
