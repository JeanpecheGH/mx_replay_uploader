use std::str::FromStr;

use xml::{EventReader, reader::XmlEvent};

use super::ParseError;

#[derive(Clone, Debug, Default)]
pub(crate) struct GbxReplayHeader {
    meta: GbxMeta,
    map: GbxMap,
    desc: GbxDesc,
    score: GbxScore,
}

#[derive(Clone, Debug, Default)]
struct GbxMeta {
    version: String,
    exever: String,
    title: String,
}

#[derive(Clone, Debug, Default)]
struct GbxMap {
    uid: String,
    name: String,
    author: String,
}

#[derive(Clone, Debug, Default)]
struct GbxDesc {
    envir: String,
    mood: String,
    maptype: String,
    mapstyle: String,
}

#[derive(Copy, Clone, Debug, Default)]
struct GbxScore {
    best: u32,
    respawns: u32,
    stuntscore: u32,
    validable: bool,
}

impl GbxReplayHeader {
    pub fn uid(&self) -> &str {
        &self.map.uid
    }

    pub fn clean_name(&self) -> String {
        let mut out_chars: Vec<char> = Vec::new();
        let mut chars = self.map.name.chars();
        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some('0'..='9' | 'a'..='f' | 'A'..='F') = chars.next() {
                    chars.next();
                    chars.next();
                }
            } else {
                out_chars.push(c);
            }
        }
        let s: String = out_chars.into_iter().collect();
        s.trim().to_string()
    }
    pub fn author(&self) -> &str {
        &self.map.author
    }
    pub fn time(&self) -> String {
        let millis: u32 = self.score.best % 1000;
        let seconds: u32 = self.score.best / 1000;
        let s: u32 = seconds % 60;
        let m: u32 = seconds / 60;
        let time: String = if m > 0 {
            format!("{m}'{s:02}.{millis:03}")
        } else {
            format!("{s}.{millis:03}")
        };
        time
    }

    /// Parses the xml header included in GBX replay
    pub(crate) fn parse_replay_xml(buf: &[u8]) -> Result<GbxReplayHeader, ParseError> {
        let xmlp = EventReader::new(buf);

        let mut header = GbxReplayHeader::default();
        let mut is_replay = false;

        for e in xmlp {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => match name.local_name.as_str() {
                    "header" => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "type" => match attr.value.as_ref() {
                                    "replay" => is_replay = true,
                                    _ => continue,
                                },
                                "version" => header.meta.version = attr.value,
                                "exever" => {
                                    header.meta.exever = attr.value;
                                }
                                "title" => {
                                    header.meta.title = attr.value;
                                }
                                _ => (),
                            }
                        }
                    }
                    "map" => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "uid" => {
                                    header.map.uid = attr.value;
                                }
                                "name" => {
                                    header.map.name = attr.value;
                                }

                                "author" => {
                                    header.map.author = attr.value;
                                }
                                _ => (),
                            }
                        }
                    }
                    "desc" => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "envir" => {
                                    header.desc.envir = attr.value;
                                }
                                "mood" => {
                                    header.desc.mood = attr.value;
                                }
                                "maptype" => {
                                    header.desc.maptype = attr.value;
                                }
                                "mapstyle" => {
                                    header.desc.mapstyle = attr.value;
                                }
                                _ => (),
                            }
                        }
                    }
                    "times" => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "best" => {
                                    header.score.best = u32::from_str(attr.value.as_str())
                                        .map_err(ParseError::HeaderValue)?
                                }
                                "respawns" => {
                                    header.score.respawns = u32::from_str(attr.value.as_str())
                                        .map_err(ParseError::HeaderValue)?
                                }
                                "stuntscore" => {
                                    header.score.stuntscore = u32::from_str(attr.value.as_str())
                                        .map_err(ParseError::HeaderValue)?
                                }
                                "validable" => {
                                    header.score.validable = 0
                                        != u32::from_str(attr.value.as_str())
                                            .map_err(ParseError::HeaderValue)?
                                }
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                },
                Err(e) => return Err(ParseError::XMLParse(e)),
                _ => {}
            }
        }

        if is_replay {
            Ok(header)
        } else {
            Err(ParseError::HeaderNotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GbxReplayHeader;

    #[test]
    fn successfull_parse() {
        let xml = br#"<header type="replay" exever="3.3.0" exebuild="2019-11-19_18_50" title="TMCanyon"><map uid="46jinRt4pKMhhaspKAwc7F2_Ek" name="$i$sSCL - $fd3Spaghetti Junction" author="helico" authorzone="World|Europe|United Kingdom|England|South West England"/><desc envir="Canyon" mood="Day" maptype="Race" mapstyle="" displaycost="6671" mod="SCL2023Official" /><playermodel id="CanyonCar"/><times best="57648" respawns="0" stuntscore="7" validable="1"/><checkpoints cur="12" onelap="12"/></header>"#;

        let hdr: GbxReplayHeader = GbxReplayHeader::parse_replay_xml(xml).unwrap();
        println!("{:#?}", hdr);
    }
}
