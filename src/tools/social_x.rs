//! X (Twitter) posting tool via X API v2.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::{Result, ZeptoError};

use super::{Tool, ToolCategory, ToolContext, ToolOutput};

const X_API_BASE: &str = "https://api.twitter.com/2";

pub struct XPostTool {
    api_key: String,
    api_secret: String,
    access_token: String,
    access_token_secret: String,
    client: Client,
}

impl XPostTool {
    pub fn new(
        api_key: &str,
        api_secret: &str,
        access_token: &str,
        access_token_secret: &str,
    ) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            access_token: access_token.to_string(),
            access_token_secret: access_token_secret.to_string(),
            client: Client::new(),
        }
    }

    fn validate_credentials(&self) -> Result<(&str, &str, &str, &str)> {
        let k = self.api_key.trim();
        let s = self.api_secret.trim();
        let t = self.access_token.trim();
        let ts = self.access_token_secret.trim();
        if k.is_empty() || s.is_empty() || t.is_empty() || ts.is_empty() {
            return Err(ZeptoError::Config(
                "X/Twitter credentials incomplete. Need api_key, api_secret, access_token, access_token_secret.".into(),
            ));
        }
        Ok((k, s, t, ts))
    }

    fn build_oauth_header(
        &self,
        method: &str,
        url: &str,
    ) -> String {
        use oauth_credentials::{Credentials, Token};
        use oauth1_request::signature_method::HmacSha1;

        let client = Credentials::new(self.api_key.trim(), self.api_secret.trim());
        let token = Credentials::new(self.access_token.trim(), self.access_token_secret.trim());
        let full_token = Token::new(client, token);

        oauth1_request::authorize(method, url, &(), &full_token, HmacSha1::new())
    }
}

#[async_trait]
impl Tool for XPostTool {
    fn name(&self) -> &str {
        "post_x"
    }

    fn description(&self) -> &str {
        "Post a tweet on X (Twitter). Text must be 280 characters or less."
    }

    fn compact_description(&self) -> &str {
        "Post on X"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Messaging
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Tweet text (max 280 characters)."
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        self.validate_credentials()?;

        let text = args["text"].as_str().unwrap_or("").to_string();
        if text.is_empty() {
            return Ok(ToolOutput::error("text is required"));
        }
        if text.len() > 280 {
            return Ok(ToolOutput::error(format!(
                "Tweet text is {} characters, exceeds 280 character limit",
                text.len()
            )));
        }

        let url = format!("{}/tweets", X_API_BASE);
        let auth_header = self.build_oauth_header("POST", &url);

        let body = json!({ "text": text });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ZeptoError::Tool(format!("X API request failed: {}", e)))?;

        let status = resp.status();
        let resp_body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Ok(ToolOutput::error(format!(
                "X API error (HTTP {}): {}",
                status, resp_body
            )));
        }

        let result: Value = serde_json::from_str(&resp_body)
            .map_err(|e| ZeptoError::Tool(format!("Failed to parse X response: {}", e)))?;

        let tweet_id = result["data"]["id"]
            .as_str()
            .unwrap_or("unknown");
        let username = result["includes"]["users"][0]["username"]
            .as_str()
            .unwrap_or("");

        Ok(ToolOutput::llm_only(format!(
            "Tweet posted successfully. ID: {}{}",
            tweet_id,
            if username.is_empty() {
                String::new()
            } else {
                format!(" by @{}", username)
            }
        )))
    }
}

pub async fn test_x_connection(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_token_secret: &str,
) -> std::result::Result<String, String> {
    let tool = XPostTool::new(api_key, api_secret, access_token, access_token_secret);
    let url = format!("{}/users/me", X_API_BASE);
    let auth_header = tool.build_oauth_header("GET", &url);

    let client = Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", auth_header)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status, body));
    }

    let me: Value = serde_json::from_str(&body).unwrap_or_default();
    let username = me["data"]["username"]
        .as_str()
        .unwrap_or("unknown");
    Ok(format!("@{}", username))
}

