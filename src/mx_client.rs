use reqwest::blocking::Client;
use reqwest::blocking::multipart::Form;
use reqwest::header;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MapId {
    map_id: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MapFromUid {
    more: bool,
    results: Vec<MapId>,
}

impl MapFromUid {
    fn id(&self) -> Option<usize> {
        self.results.first().map(|r| r.map_id)
    }
}

pub struct MxClient {
    client: Client,
}

impl MxClient {
    pub fn build_mx_client() -> MxClient {
        let client: Client = Client::builder()
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .cookie_store(true)
            .build()
            .unwrap();
        MxClient { client }
    }

    /// Extract the next action
    fn extract_next_action(
        &self,
        html: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let document = Html::parse_document(html);

        // Select hidden inputs within forms
        let selector = Selector::parse(r#"form[method="post"]"#)?;

        if let Some(element) = document.select(&selector).next() {
            Ok(element.value().attr("action").map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Extract all hidden input fields from the login form
    fn extract_hidden_fields(
        &self,
        html: &str,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let document = Html::parse_document(html);

        // Select hidden inputs within forms
        let selector = Selector::parse(r#"form input[type="hidden"]"#)?;

        let mut fields = HashMap::new();

        for element in document.select(&selector) {
            if let (Some(name), Some(value)) =
                (element.value().attr("name"), element.value().attr("value"))
            {
                fields.insert(name.to_string(), value.to_string());
            }
        }

        Ok(fields)
    }

    pub fn authenticate(&self, user: &str, pwd: &str) {
        // GET the login page to obtain the hidden form fields
        let response = self
            .client
            .get("https://account.mania.exchange/login")
            .send();
        let text = response.unwrap().text().unwrap();

        let mut params = self.extract_hidden_fields(&text).unwrap();
        params.insert(String::from("Username"), String::from(user));
        params.insert(String::from("Password"), String::from(pwd));
        params.insert(String::from("button"), String::from("login"));

        // POST the login form
        let response = self
            .client
            .post("https://account.mania.exchange/login")
            .form(&params)
            .header(header::ORIGIN, "null")
            .send();
        let mut text = response.unwrap().text().unwrap();
        let mut previous: String = "https://account.mania.exchange/login".to_string();

        // Chain of POST pseudo redirects
        // While we get action=next_url
        // Get hidden token & fdata form values
        while let Ok(Some(action)) = self.extract_next_action(&text) {
            println!("Action: {:?}", action);
            let params = self.extract_hidden_fields(&text).unwrap();
            text = self
                .client
                .post(action.clone())
                .form(&params)
                .header(header::ORIGIN, previous)
                .send()
                .unwrap()
                .text()
                .unwrap();
            previous = action;
        }
    }

    pub fn get_account(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client.get("https://tm.mania.exchange/login").send()?;
        println!(
            "Mania-exchange login to https://tm.mania.exchange/login: {}",
            response.status()
        );
        let text = response.text()?;

        if let Ok(Some(action)) = self.extract_next_action(&text) {
            let params = self.extract_hidden_fields(&text).unwrap();
            let response = self.client.post(action.clone()).form(&params).send()?;
            println!("Action {}: {}", action, response.status());
            let _ = response.text()?;
        }

        let response = self.client.get("https://tm.mania.exchange").send()?;
        println!(
            "Final test to https://tm.mania.exchange: {}",
            response.status()
        );
        let text = response.text()?;

        Ok(text)
    }

    pub fn get_map_id(&self, uid: &str) -> Result<Option<usize>, Box<dyn std::error::Error>> {
        let url = "https://tm.mania.exchange/api/maps";
        let params = [("fields", "MapId"), ("uid", uid)];
        let url = reqwest::Url::parse_with_params(url, &params)?;
        let json = self.client.get(url).send()?.json::<MapFromUid>()?;
        Ok(json.id())
    }

    pub fn upload_replay(&self, path: &str, id: usize) -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://tm.mania.exchange/api/replays/upload";
        let referer = format!("https://tm.mania.exchange/replayupload/{}", id);

        let response = self.client.get(referer.clone()).send()?;
        println!("Get map {}: {}", referer, response.status());
        let text = response.text()?;

        let params = self.extract_hidden_fields(&text)?;
        let a: String = params.keys().next().unwrap().to_string();
        let b: String = params.values().next().unwrap().to_string();
        let form = Form::new().text(a, b).file("file", path)?;
        let response = self
            .client
            .post(url)
            .multipart(form)
            .header(header::REFERER, referer) // Some sites check referer
            .header(header::ORIGIN, "tm.mania.exchange")
            .send()?
            .text()?;
        println!("Response POST: {:?}", response);
        Ok(())
    }
}
