use std::fmt::{Display, Formatter, Result};

pub struct Log {
    pub kind: LogKind,
    pub value: String,
}

impl Display for Log {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "[{}] {}", self.kind, self.value)
    }
}

pub enum LogKind {
    Client,
    Record,
    Upload,
    Watcher,
}

impl Display for LogKind {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            LogKind::Client => write!(f, "Client"),
            LogKind::Record => write!(f, "Record"),
            LogKind::Upload => write!(f, "Upload"),
            LogKind::Watcher => write!(f, "Watcher"),
        }
    }
}
