use notify::{Event, RecursiveMode, Result, Watcher};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub fn watch_folder(path: &str, sender: Sender<Result<Event>>) -> Result<()> {
    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(sender)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

    // This thread must block indefinitely to keep watching
    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
