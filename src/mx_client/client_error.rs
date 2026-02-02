use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum ClientError {
    Error(String),
    Upload(String),
    Reqwest(reqwest::Error),
    HtmlParse(String),
    NoMapId,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ClientError::Error(e) => write!(f, "Client error: {}", e),
            ClientError::Upload(e) => write!(f, "Upload error: {}", e),
            ClientError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            ClientError::HtmlParse(e) => write!(f, "Html parse error: {}", e),
            ClientError::NoMapId => write!(f, "Map does not exist on Mania Exchange"),
        }
    }
}
