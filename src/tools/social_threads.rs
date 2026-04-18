//! Threads (Meta) posting tool via Threads API.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::{Result, ZeptoError};

use super::{Tool, ToolCategory, ToolContext, ToolOutput};

const THREADS_API_BASE: &str = "https://graph.threads.net/v1.0";

pub struct ThreadsPostTool {
    user_id: String,
    access_token: String,
    client: Client,
}

impl ThreadsPostTool {
    pub fn new(user_id: &str, access_token: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            access_token: access_token.to_string(),
            client: Client::new(),
        }
    }

    pub fn validate_credentials(&self) -> Result<(String, String)> {
        let user_id = self.user_id.trim();
        let token = self.access_token.trim();
        if user_id.is_empty() {
            return Err(ZeptoError::Config("Threads user_id is not configured".into()));
        }
        if token.is_empty() {
            return Err(ZeptoError::Config(
                "Threads access_token is not configured".into(),
            ));
        }
        Ok((user_id.to_string(), token.to_string()))
    }
}

#[async_trait]
impl Tool for ThreadsPostTool {
    fn name(&self) -> &str {
        "post_threads"
    }

    fn description(&self) -> &str {
        "Post to Threads (Meta). Supports text posts (up to 500 chars), image posts, video posts, and link attachments."
    }

    fn compact_description(&self) -> &str {
        "Post to Threads"
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
                    "description": "Post text content. Required for text posts. Up to 500 characters. Emojis count as UTF-8 bytes."
                },
                "media_type": {
                    "type": "string",
                    "enum": ["TEXT", "IMAGE", "VIDEO"],
                    "description": "Type of media for the post. Default: TEXT."
                },
                "image_url": {
                    "type": "string",
                    "description": "Public URL of an image to attach. Required when media_type is IMAGE."
                },
                "video_url": {
                    "type": "string",
                    "description": "Public URL of a video to attach. Required when media_type is VIDEO."
                },
                "link_attachment": {
                    "type": "string",
                    "description": "URL to attach as a link preview. Only works with TEXT media_type."
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let (user_id, token) = self.validate_credentials()?;

        let text = args["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if text.is_empty() {
            return Ok(ToolOutput::error("text is required"));
        }

        let media_type = args["media_type"]
            .as_str()
            .unwrap_or("TEXT");
        let image_url = args["image_url"].as_str().unwrap_or("");
        let video_url = args["video_url"].as_str().unwrap_or("");
        let link_attachment = args["link_attachment"].as_str().unwrap_or("");

        let mut form = vec![("access_token", token.as_str())];

        match media_type {
            "IMAGE" => {
                if image_url.is_empty() {
                    return Ok(ToolOutput::error(
                        "image_url is required when media_type is IMAGE",
                    ));
                }
                form.push(("media_type", "IMAGE"));
                form.push(("image_url", image_url));
                if !text.is_empty() {
                    form.push(("text", &text));
                }
            }
            "VIDEO" => {
                if video_url.is_empty() {
                    return Ok(ToolOutput::error(
                        "video_url is required when media_type is VIDEO",
                    ));
                }
                form.push(("media_type", "VIDEO"));
                form.push(("video_url", video_url));
                if !text.is_empty() {
                    form.push(("text", &text));
                }
            }
            _ => {
                form.push(("media_type", "TEXT"));
                form.push(("text", &text));
            }
        }

        if !link_attachment.is_empty() && media_type == "TEXT" {
            form.push(("link_attachment", link_attachment));
        }

        let create_url = format!("{}/{}/threads", THREADS_API_BASE, user_id);
        let resp = self
            .client
            .post(&create_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| ZeptoError::Tool(format!("Threads API request failed: {}", e)))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Ok(ToolOutput::error(format!(
                "Threads create failed (HTTP {}): {}",
                status, body
            )));
        }

        let container_id: Value =
            serde_json::from_str(&body).map_err(|e| ZeptoError::Tool(format!("Failed to parse Threads response: {}", e)))?;

        let id = container_id["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let publish_url = format!("{}/{}/threads_publish", THREADS_API_BASE, user_id);
        let pub_resp = self
            .client
            .post(&publish_url)
            .form(&[
                ("creation_id", id.as_str()),
                ("access_token", token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| ZeptoError::Tool(format!("Threads publish request failed: {}", e)))?;

        let pub_status = pub_resp.status();
        let pub_body = pub_resp.text().await.unwrap_or_default();

        if !pub_status.is_success() {
            return Ok(ToolOutput::error(format!(
                "Threads publish failed (HTTP {}): {}",
                pub_status, pub_body
            )));
        }

        let pub_result: Value = serde_json::from_str(&pub_body)
            .map_err(|e| ZeptoError::Tool(format!("Failed to parse Threads publish response: {}", e)))?;

        let media_id = pub_result["id"].as_str().unwrap_or("unknown");

        Ok(ToolOutput::llm_only(format!(
            "Posted to Threads successfully. Media ID: {}",
            media_id
        )))
    }
}

pub async fn test_threads_connection(
    user_id: &str,
    access_token: &str,
) -> std::result::Result<String, String> {
    let client = Client::new();
    let url = format!(
        "{}/me?fields=id,username&access_token={}",
        THREADS_API_BASE, access_token
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status, body));
    }

    let me: Value = serde_json::from_str(&body).unwrap_or_default();
    let username = me["username"].as_str().unwrap_or(user_id);
    Ok(username.to_string())
}
