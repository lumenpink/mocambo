use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct FuxicoShareRequest {
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub mocambo_token: String,
    pub mocambo_stream_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FuxicoShareResponse {
    pub slug: String,
    pub url: String,
    pub x_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FuxicoDeleteResponse {
    pub success: bool,
    pub slug: Option<String>,
}

pub struct FuxicoClient {
    client: Client,
    base_url: String,
    token: String,
}

impl FuxicoClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        }
    }

    pub async fn create_share(
        &self,
        req: &FuxicoShareRequest,
    ) -> Result<FuxicoShareResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/mocambo/share", self.base_url);
        let mut builder = self.client.post(&url).json(req);
        if !self.token.is_empty() {
            builder = builder.bearer_auth(&self.token);
        }

        let resp = builder.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Fuxico create share failed ({status}): {body}").into());
        }

        let data = resp.json::<FuxicoShareResponse>().await?;
        Ok(data)
    }

    pub async fn revoke_share(
        &self,
        slug: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/mocambo/share/{}", self.base_url, slug);
        let mut builder = self.client.delete(&url);
        if !self.token.is_empty() {
            builder = builder.bearer_auth(&self.token);
        }

        let resp = builder.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Fuxico revoke share failed ({status}): {body}").into());
        }

        let data = resp.json::<FuxicoDeleteResponse>().await?;
        Ok(data.success)
    }
}
