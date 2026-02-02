pub mod client_error;
pub mod upload_response;

use crate::gbx_parser::GbxHeader;
use crate::mx_client::client_error::ClientError;
use crate::mx_client::upload_response::UploadResponse;
use reqwest::Client;
use reqwest::header;
use reqwest::multipart::Form;
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

#[derive(Clone)]
pub struct MxClient {
    client: Client,
}

impl MxClient {
    pub async fn connect(&self, user: &str, pwd: &str) -> Result<(), ClientError> {
        self.authenticate(user, pwd).await?;
        self.get_account().await?;
        Ok(())
    }

    pub fn build_mx_client() -> Result<MxClient, reqwest::Error> {
        Client::builder()
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .cookie_store(true)
            .build()
            .map(|client| MxClient { client })
    }

    /// Extract the next action
    fn extract_next_action(&self, html: &str) -> Result<Option<String>, ClientError> {
        let document = Html::parse_document(html);

        // Select hidden inputs within forms
        let selector = Selector::parse(r#"form[method="post"]"#)
            .map_err(|e| ClientError::HtmlParse(e.to_string()))?;

        if let Some(element) = document.select(&selector).next() {
            Ok(element.value().attr("action").map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Extract all hidden input fields from the login form
    fn extract_hidden_fields(&self, html: &str) -> Result<HashMap<String, String>, ClientError> {
        let document = Html::parse_document(html);

        // Select hidden inputs within forms
        let selector = Selector::parse(r#"form input[type="hidden"]"#)
            .map_err(|e| ClientError::HtmlParse(e.to_string()))?;

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

    async fn authenticate(&self, user: &str, pwd: &str) -> Result<(), ClientError> {
        // GET the login page to obtain the hidden form fields
        let response = self
            .client
            .get("https://account.mania.exchange/login")
            .send()
            .await
            .map_err(ClientError::Reqwest)?;
        let text = response.text().await.map_err(ClientError::Reqwest)?;

        let mut params = self.extract_hidden_fields(&text)?;
        params.insert(String::from("Username"), String::from(user));
        params.insert(String::from("Password"), String::from(pwd));
        params.insert(String::from("button"), String::from("login"));

        // POST the login form
        let response = self
            .client
            .post("https://account.mania.exchange/login")
            .form(&params)
            .header(header::ORIGIN, "null")
            .send()
            .await
            .map_err(ClientError::Reqwest)?;
        let mut text = response.text().await.map_err(ClientError::Reqwest)?;
        let mut previous: String = "https://account.mania.exchange/login".to_string();

        // Chain of POST pseudo redirects
        // While we get action=next_url
        // Get hidden token & fdata form values
        while let Ok(Some(action)) = self.extract_next_action(&text) {
            let params = self.extract_hidden_fields(&text)?;
            text = self
                .client
                .post(action.clone())
                .form(&params)
                .header(header::ORIGIN, previous)
                .send()
                .await
                .map_err(ClientError::Reqwest)?
                .text()
                .await
                .map_err(ClientError::Reqwest)?;
            previous = action;
        }

        Ok(())
    }

    async fn get_account(&self) -> Result<(), ClientError> {
        let response = self
            .client
            .get("https://tm.mania.exchange/login")
            .send()
            .await
            .map_err(ClientError::Reqwest)?;
        let text = response.text().await.map_err(ClientError::Reqwest)?;

        if let Ok(Some(action)) = self.extract_next_action(&text) {
            let params = self.extract_hidden_fields(&text)?;
            let response = self
                .client
                .post(action.clone())
                .form(&params)
                .send()
                .await
                .map_err(ClientError::Reqwest)?;
            let _ = response.text().await.map_err(ClientError::Reqwest)?;
        }

        Ok(())
    }

    pub async fn get_map_id(&self, uid: &str) -> Result<usize, ClientError> {
        let url = "https://tm.mania.exchange/api/maps";
        let params = [("fields", "MapId"), ("uid", uid)];
        let url = reqwest::Url::parse_with_params(url, &params)
            .map_err(|e| ClientError::Error(e.to_string()))?;
        let json = self
            .client
            .get(url)
            .send()
            .await
            .map_err(ClientError::Reqwest)?
            .json::<MapFromUid>()
            .await
            .map_err(ClientError::Reqwest)?;
        json.id().ok_or(ClientError::NoMapId)
    }

    pub async fn upload_replay(
        &self,
        path: &str,
        id: usize,
    ) -> Result<UploadResponse, ClientError> {
        let url = "https://tm.mania.exchange/api/replays/upload";
        let referer = format!("https://tm.mania.exchange/replayupload/{}", id);

        let response = self
            .client
            .get(referer.clone())
            .send()
            .await
            .map_err(ClientError::Reqwest)?;
        let text = response.text().await.map_err(ClientError::Reqwest)?;

        let params = self.extract_hidden_fields(&text)?;
        let (key, value) = params.into_iter().next().ok_or(ClientError::Error(format!(
            "No hidden fields found in {referer} response"
        )))?;
        let form = Form::new()
            .text(key, value)
            .file("file", path)
            .await
            .map_err(|e| ClientError::Error(e.to_string()))?;
        self.client
            .post(url)
            .multipart(form)
            .header(header::REFERER, referer) // Some sites check referer
            .header(header::ORIGIN, "tm.mania.exchange")
            .send()
            .await
            .map_err(ClientError::Reqwest)?
            .json::<UploadResponse>()
            .await
            .map_err(ClientError::Reqwest)
    }

    pub async fn upload_single(&self, replay: &GbxHeader) -> Result<UploadResponse, ClientError> {
        let id = self.get_map_id(replay.uid()).await?;
        let r = self.upload_replay(&replay.path, id).await?;
        if r.success {
            Ok(r)
        } else {
            Err(ClientError::Upload(
                r.error.unwrap_or(String::from("No error message")),
            ))
        }
    }

    pub async fn upload_all(
        &self,
        replays: Vec<GbxHeader>,
    ) -> Vec<(Result<UploadResponse, ClientError>, GbxHeader)> {
        let mut results: Vec<(Result<UploadResponse, ClientError>, GbxHeader)> = Vec::new();
        for r in replays {
            let res = self.upload_single(&r).await;
            results.push((res, r));
        }
        results
    }
}
