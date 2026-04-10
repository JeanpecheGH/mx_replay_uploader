mod replay_parser;

use crate::gbx_parser::replay_parser::GbxReplayHeader;
use std::convert::TryInto;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::Read;
use std::{fs::File, num::ParseIntError};

const HEADER_START_TOKEN: &[u8] = "<header ".as_bytes();
const HEADER_END_TOKEN: &[u8] = "</header>".as_bytes();

#[derive(Debug)]
pub enum ParseError {
    MissingGBXMagic,
    FileTooShort,
    HeaderNotFound,
    XMLParse(xml::reader::Error),
    HeaderValue(ParseIntError),
    IO(io::Error),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingGBXMagic => write!(f, "Missing GBX Magic"),
            ParseError::FileTooShort => write!(f, "File is too short"),
            ParseError::HeaderNotFound => write!(f, "Header not found"),
            ParseError::XMLParse(e) => write!(f, "XML parsing error: {e}"),
            ParseError::HeaderValue(e) => write!(f, "Header value error: {e}"),
            ParseError::IO(e) => write!(f, "IO error: {e}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GbxHeader {
    pub path: String,
    name: String,
    time: String,
    _binary_header: GbxBinaryHeader,
    replay_header: GbxReplayHeader,
}

impl GbxHeader {
    pub fn uid(&self) -> &str {
        self.replay_header.uid()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn author(&self) -> &str {
        self.replay_header.author()
    }

    pub fn time(&self) -> &str {
        &self.time
    }
}

#[derive(Debug, Clone)]
struct GbxBinaryHeader {
    _version: u16,
    _class_id: u32,
}

/// Reads the contents from `filename` and parses them identically to [parse_from_buffer](parse_from_buffer).
///
/// Note, that the [GBXOrigin](GBXOrigin) of the returned [GBX](GBX) will be `File{path:<filepath>}`.
pub fn parse_from_file(filename: &str) -> Result<GbxHeader, ParseError> {
    let mut buffer = Vec::new();
    let mut f = File::open(filename).map_err(ParseError::IO)?;
    f.read_to_end(&mut buffer).map_err(ParseError::IO)?;
    let (_binary_header, replay_header) = parse_from_buffer(&buffer)?;
    Ok(GbxHeader {
        path: String::from(filename),
        name: replay_header.clean_name(),
        time: replay_header.time(),
        _binary_header,
        replay_header,
    })
}

/// Parses the given slice of bytes as if it was a GBX file.
///
/// This function assumes the XML header included in the GBX file is valid UTF8, and will panic
/// otherwise.
/// As of now the actual map-data is not extracted.
///
/// If you want to parse a file directly see [parse_from_file](parse_from_file).
fn parse_from_buffer(buffer: &[u8]) -> Result<(GbxBinaryHeader, GbxReplayHeader), ParseError> {
    if buffer.len() < 3 {
        return Err(ParseError::FileTooShort);
    }

    if &buffer[0..3] != b"GBX" {
        return Err(ParseError::MissingGBXMagic);
    }

    let binary_header = GbxBinaryHeader {
        _version: u16::from_le_bytes((&buffer[3..5]).try_into().unwrap()),
        _class_id: u32::from_le_bytes((&buffer[9..13]).try_into().unwrap()),
    };

    let header_start: usize = find_window(&buffer[13..], HEADER_START_TOKEN)
        .ok_or(ParseError::HeaderNotFound)
        .map(|x| x + 13)?;
    let header_end = find_window(&buffer[header_start..], HEADER_END_TOKEN)
        .ok_or(ParseError::HeaderNotFound)
        .map(|x| x + header_start + HEADER_END_TOKEN.len())?;

    let replay_header: GbxReplayHeader =
        GbxReplayHeader::parse_replay_xml(&buffer[header_start..header_end])?;

    Ok((binary_header, replay_header))
}

/// Util. function to find first match of a sub-slice in a slice
fn find_window(buf: &[u8], needle: &[u8]) -> Option<usize> {
    buf.windows(needle.len()).position(|w| w == needle)
}
