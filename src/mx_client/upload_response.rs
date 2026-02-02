//Response POST: "{"Success":true,"Map":{"MapId":41496,"Name":"TheseRoadS"},"Replay":{"ReplayTime":43825,"ReplayPoints":26,"Position":2,"Score":935},"WRReplay":{"ReplayTime":43415,"ReplayPoints":90}}"
// ERROR {"Success":false,"Error":"This replay already exists.","Map":{"MapId":260354,"Name":" NoTimeToLift"},"Replay":{"ReplayTime":58241,"ReplayPoints":6,"Position":0,"Score":1000}}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub success: bool,
    pub error: Option<String>,
    map: Map,
    replay: Replay,
}

impl UploadResponse {
    pub fn position(&self) -> usize {
        self.replay.position + 1
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Map {
    map_id: usize,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Replay {
    replay_time: usize,
    replay_points: usize,
    position: usize,
    score: usize,
}
