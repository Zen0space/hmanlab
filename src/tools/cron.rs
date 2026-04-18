//! Cron scheduling tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::cron::{
    is_valid_cron_expr, parse_at_datetime_ms, CronPayload, CronSchedule, CronService,
};
use crate::error::{Result, ZeptoError};

use super::{Tool, ToolCategory, ToolContext, ToolOutput};

/// Tool for creating and managing scheduled jobs.
pub struct CronTool {
    cron: Arc<CronService>,
}

impl CronTool {
    /// Create a new cron tool.
    pub fn new(cron: Arc<CronService>) -> Self {
        Self { cron }
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Schedule recurring tasks that run through the agent with full tool access (web_search, web_fetch, etc). \
         The message is a prompt the agent executes — write it as an instruction describing what to do. \
         Examples: 'Search for the latest market news and summarize them', \
         'Fetch the current weather for London and post a brief report'. \
         Supports {{date}}, {{time}}, {{datetime}} template variables. \
         Actions: add, list, remove."
    }

    fn compact_description(&self) -> &str {
        "Schedule recurring agent task with tool access (web_search, web_fetch, etc)"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Destructive
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "list", "remove"],
                    "description": "Action to perform"
                },
                "message": {
                    "type": "string",
                    "description": "Prompt the agent will execute when the job fires. Include tool instructions like 'search the web for...' or 'fetch https://...'. Supports {{date}}, {{time}}, {{datetime}}."
                },
                "name": {
                    "type": "string",
                    "description": "Optional job name"
                },
                "every_seconds": {
                    "type": "integer",
                    "description": "Run interval in seconds"
                },
                "cron_expr": {
                    "type": "string",
                    "description": "Cron expression (UTC)"
                },
                "at": {
                    "type": "string",
                    "description": "One-shot ISO datetime"
                },
                "job_id": {
                    "type": "string",
                    "description": "Target job id for remove"
                },
                "channel": {
                    "type": "string",
                    "description": "Optional target channel (defaults to current)"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Optional target chat id (defaults to current)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ZeptoError::Tool("Missing 'action' argument".into()))?;

        let s = match action {
            "add" => self.execute_add(args, ctx).await?,
            "list" => self.execute_list(args).await?,
            "remove" => self.execute_remove(args).await?,
            other => return Err(ZeptoError::Tool(format!("Unknown cron action '{}'", other))),
        };
        Ok(ToolOutput::llm_only(s))
    }
}

fn enrich_cron_message(message: &str) -> String {
    static KEYWORDS: &[&str] = &[
        "search", "fetch", "browse", "check", "monitor", "scrape", "look up", "news", "weather",
        "price", "stock", "update", "latest", "current", "summary", "report",
    ];

    let lower = message.to_lowercase();
    let needs_tools = KEYWORDS.iter().any(|kw| lower.contains(kw));

    if !needs_tools {
        return message.to_string();
    }

    let already_explicit = lower.contains("use web_search")
        || lower.contains("use web_fetch")
        || lower.contains("use the available tools")
        || lower.contains("use your tools");

    if already_explicit {
        return format!(
            "[Scheduled task - execute autonomously, do not ask follow-up questions]\n{}",
            message
        );
    }

    format!(
        "[Scheduled task - execute autonomously, do not ask follow-up questions]\n{}\n\n\
         Use the available tools (web_search, web_fetch) to find current and accurate information, \
         then post a concise summary of your findings.",
        message
    )
}

impl CronTool {
    async fn execute_add(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let existing = self.cron.list_jobs(false).await;
        if existing.len() >= 50 {
            return Err(ZeptoError::Tool(
                "Maximum of 50 active cron jobs reached. Remove some before adding new ones."
                    .to_string(),
            ));
        }

        let raw_message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ZeptoError::Tool("Missing 'message' for cron add".into()))?;

        let message = enrich_cron_message(raw_message);

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| {
                if raw_message.chars().count() > 30 {
                    let end = raw_message
                        .char_indices()
                        .nth(30)
                        .map(|(i, _)| i)
                        .unwrap_or(raw_message.len());
                    format!("{}...", &raw_message[..end])
                } else {
                    raw_message.to_string()
                }
            });

        let every_seconds = args.get("every_seconds").and_then(|v| v.as_i64());
        let cron_expr = args.get("cron_expr").and_then(|v| v.as_str());
        let at = args.get("at").and_then(|v| v.as_str());

        let mut schedule_count = 0;
        if every_seconds.is_some() {
            schedule_count += 1;
        }
        if cron_expr.is_some() {
            schedule_count += 1;
        }
        if at.is_some() {
            schedule_count += 1;
        }
        if schedule_count != 1 {
            return Err(ZeptoError::Tool(
                "Specify exactly one of: every_seconds, cron_expr, at".to_string(),
            ));
        }

        // Minimum interval rate limiting
        if let Some(seconds) = every_seconds {
            if seconds < 60 {
                return Err(ZeptoError::Tool(
                    "Minimum interval is 60 seconds".to_string(),
                ));
            }
        }

        let (schedule, delete_after_run) = if let Some(seconds) = every_seconds {
            if seconds <= 0 {
                return Err(ZeptoError::Tool(
                    "'every_seconds' must be greater than zero".to_string(),
                ));
            }
            (
                CronSchedule::Every {
                    every_ms: seconds * 1_000,
                },
                false,
            )
        } else if let Some(expr) = cron_expr {
            let schedule = CronSchedule::Cron {
                expr: expr.to_string(),
            };
            if !is_valid_cron_expr(expr) {
                return Err(ZeptoError::Tool(format!(
                    "Invalid or non-runnable cron expression '{}'",
                    expr
                )));
            }
            (schedule, false)
        } else {
            let at_ms = parse_at_datetime_ms(at.unwrap())?;
            (CronSchedule::At { at_ms }, true)
        };

        let channel = args
            .get("channel")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| ctx.channel.clone())
            .ok_or_else(|| ZeptoError::Tool("No channel available in tool context".into()))?;

        let chat_id = args
            .get("chat_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| ctx.chat_id.clone())
            .ok_or_else(|| ZeptoError::Tool("No chat_id available in tool context".into()))?;

        let job = self
            .cron
            .add_job(
                name,
                schedule,
                CronPayload {
                    message: message.to_string(),
                    channel,
                    chat_id,
                },
                delete_after_run,
            )
            .await?;

        Ok(format!("Created cron job '{}' (id: {})", job.name, job.id))
    }

    async fn execute_list(&self, args: Value) -> Result<String> {
        let include_disabled = args
            .get("include_disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let jobs = self.cron.list_jobs(include_disabled).await;
        if jobs.is_empty() {
            return Ok("No scheduled jobs".to_string());
        }

        let mut lines = Vec::new();
        for job in jobs {
            let schedule = match &job.schedule {
                CronSchedule::At { at_ms } => format!("at({})", at_ms),
                CronSchedule::Every { every_ms } => format!("every({}ms)", every_ms),
                CronSchedule::Cron { expr } => format!("cron({})", expr),
            };
            lines.push(format!(
                "- {} [{}] {} -> {}:{}",
                job.name, job.id, schedule, job.payload.channel, job.payload.chat_id
            ));
        }
        Ok(format!("Scheduled jobs:\n{}", lines.join("\n")))
    }

    async fn execute_remove(&self, args: Value) -> Result<String> {
        let job_id = args
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ZeptoError::Tool("Missing 'job_id' for cron remove".into()))?;

        if self.cron.remove_job(job_id).await? {
            Ok(format!("Removed cron job {}", job_id))
        } else {
            Ok(format!("Cron job {} not found", job_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::MessageBus;
    use crate::cron::CronService;
    use tempfile::tempdir;

    /// Helper: create a CronTool backed by a temp-dir CronService.
    fn make_cron_tool() -> CronTool {
        let temp = tempdir().expect("failed to create temp dir");
        let bus = Arc::new(MessageBus::new());
        let service = Arc::new(CronService::new(temp.path().join("jobs.json"), bus));
        CronTool::new(service)
    }

    /// Helper: build a ToolContext with channel and chat_id set.
    fn ctx_with_channel() -> ToolContext {
        ToolContext::new().with_channel("telegram", "chat_42")
    }

    // ---- metadata tests ----

    #[test]
    fn test_cron_tool_name() {
        let tool = make_cron_tool();
        assert_eq!(tool.name(), "cron");
    }

    #[test]
    fn test_cron_tool_description() {
        let tool = make_cron_tool();
        assert!(tool.description().contains("Schedule"));
        assert!(tool.description().contains("add"));
        assert!(tool.description().contains("list"));
        assert!(tool.description().contains("remove"));
    }

    #[test]
    fn test_cron_tool_parameters_schema() {
        let tool = make_cron_tool();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["action"].is_object());
        assert!(params["properties"]["message"].is_object());
        assert!(params["properties"]["every_seconds"].is_object());
        assert!(params["properties"]["cron_expr"].is_object());
        assert!(params["properties"]["at"].is_object());
        assert!(params["properties"]["job_id"].is_object());
        assert_eq!(params["required"], json!(["action"]));
    }

    // ---- execute tests ----

    #[tokio::test]
    async fn test_execute_missing_action() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool.execute(json!({}), &ctx).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing 'action'"));
    }

    #[tokio::test]
    async fn test_execute_invalid_action() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool.execute(json!({"action": "restart"}), &ctx).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unknown cron action 'restart'"));
    }

    #[tokio::test]
    async fn test_execute_add_missing_message() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool
            .execute(json!({"action": "add", "every_seconds": 120}), &ctx)
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing 'message'"));
    }

    #[tokio::test]
    async fn test_execute_add_no_schedule() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        // Provide a message but no schedule type at all.
        let result = tool
            .execute(json!({"action": "add", "message": "hello"}), &ctx)
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Specify exactly one"));
    }

    #[tokio::test]
    async fn test_execute_add_multiple_schedules() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool
            .execute(
                json!({
                    "action": "add",
                    "message": "hello",
                    "every_seconds": 120,
                    "cron_expr": "*/5 * * * *"
                }),
                &ctx,
            )
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Specify exactly one"));
    }

    #[tokio::test]
    async fn test_execute_add_interval_too_short() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool
            .execute(
                json!({
                    "action": "add",
                    "message": "ping",
                    "every_seconds": 10
                }),
                &ctx,
            )
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Minimum interval is 60 seconds"));
    }

    #[tokio::test]
    async fn test_execute_add_every_seconds_success() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool
            .execute(
                json!({
                    "action": "add",
                    "message": "heartbeat",
                    "every_seconds": 120
                }),
                &ctx,
            )
            .await;
        assert!(result.is_ok());
        let output = result.unwrap().for_llm;
        assert!(output.contains("Created cron job"));
        assert!(output.contains("heartbeat"));
    }

    #[tokio::test]
    async fn test_execute_list_empty() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool.execute(json!({"action": "list"}), &ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().for_llm, "No scheduled jobs");
    }

    #[tokio::test]
    async fn test_execute_remove_missing_job_id() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool.execute(json!({"action": "remove"}), &ctx).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing 'job_id'"));
    }

    #[tokio::test]
    async fn test_execute_remove_nonexistent_job() {
        let tool = make_cron_tool();
        let ctx = ctx_with_channel();

        let result = tool
            .execute(json!({"action": "remove", "job_id": "no_such_id"}), &ctx)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().for_llm.contains("not found"));
    }

    #[tokio::test]
    async fn test_execute_add_no_channel_in_context() {
        let tool = make_cron_tool();
        let ctx = ToolContext::new(); // no channel or chat_id

        let result = tool
            .execute(
                json!({
                    "action": "add",
                    "message": "test",
                    "every_seconds": 120
                }),
                &ctx,
            )
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No channel available"));
    }

    #[test]
    fn test_enrich_cron_message_with_search_keyword() {
        let result = enrich_cron_message("Fetch the latest market news");
        assert!(result.starts_with("[Scheduled task - execute autonomously"));
        assert!(result.contains("Fetch the latest market news"));
        assert!(result.contains("web_search"));
        assert!(result.contains("web_fetch"));
    }

    #[test]
    fn test_enrich_cron_message_with_stock_keyword() {
        let result = enrich_cron_message("Check AAPL stock price");
        assert!(result.starts_with("[Scheduled task - execute autonomously"));
        assert!(result.contains("Check AAPL stock price"));
    }

    #[test]
    fn test_enrich_cron_message_plain_text_unchanged() {
        let msg = "Send a good morning greeting";
        let result = enrich_cron_message(msg);
        assert_eq!(result, msg);
    }

    #[test]
    fn test_enrich_cron_message_already_has_tool_instruction() {
        let result = enrich_cron_message("Use web_search to find market news");
        assert!(result.starts_with("[Scheduled task - execute autonomously"));
        assert!(result.contains("Use web_search to find market news"));
        assert!(
            !result.contains("Use the available tools"),
            "should not double-add tool instructions"
        );
    }

    #[test]
    fn test_enrich_cron_message_case_insensitive() {
        let result = enrich_cron_message("GET THE LATEST UPDATES");
        assert!(result.starts_with("[Scheduled task - execute autonomously"));
    }
}
