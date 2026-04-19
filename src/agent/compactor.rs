use crate::error::AppError;
use crate::llm::{GenerateRequest, LlmProvider, Message};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::types::ChatMessage;

pub trait HistoryCompactor: Send + Sync {
    fn compact<'a>(
        &'a self,
        messages: &'a [ChatMessage],
    ) -> Pin<Box<dyn Future<Output = Result<ChatMessage, AppError>> + Send + 'a>>;
}

const DEFAULT_SUMMARY_PROMPT: &str = "Summarize the following conversation turns between a user and an assistant. Preserve:\n\
- The user's goals, constraints, and preferences.\n\
- Key facts established.\n\
- Tool calls made and their outcomes, as facts (not verbatim).\n\
- Decisions or commitments made.\n\
\n\
Write in third person (\"The user asked...\", \"The assistant found...\") as a dense paragraph. \
Do not reproduce the dialog. Under 200 words.";

pub struct LlmSummaryCompactor {
    provider: Arc<dyn LlmProvider>,
    model: String,
    prompt: String,
    max_tokens: u32,
}

impl LlmSummaryCompactor {
    pub fn new(provider: Arc<dyn LlmProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            prompt: DEFAULT_SUMMARY_PROMPT.into(),
            max_tokens: 500,
        }
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    pub fn with_max_tokens(mut self, v: u32) -> Self {
        self.max_tokens = v;
        self
    }
}

impl HistoryCompactor for LlmSummaryCompactor {
    fn compact<'a>(
        &'a self,
        messages: &'a [ChatMessage],
    ) -> Pin<Box<dyn Future<Output = Result<ChatMessage, AppError>> + Send + 'a>> {
        Box::pin(async move {
            let rendered = render_for_summary(messages);
            let prompt = format!("{}\n\nConversation:\n{rendered}", self.prompt);

            let req = GenerateRequest::new(&self.model, vec![Message::user(prompt)])
                .with_max_tokens(self.max_tokens)
                .with_temperature(0.2);

            let resp = self.provider.generate(&req).await?;
            let summary = resp
                .text()
                .unwrap_or_else(|| "[summary unavailable]".into());

            Ok(ChatMessage::Assistant {
                content: format!("[Prior conversation summary]\n{summary}"),
            })
        })
    }
}

fn render_for_summary(messages: &[ChatMessage]) -> String {
    let mut out = String::with_capacity(messages.len() * 64);
    for m in messages {
        match m {
            ChatMessage::User { content } => {
                out.push_str("User: ");
                out.push_str(content);
                out.push('\n');
            }
            ChatMessage::Assistant { content } => {
                out.push_str("Assistant: ");
                out.push_str(content);
                out.push('\n');
            }
            ChatMessage::ToolCall { name, args, .. } => {
                out.push_str(&format!("Tool call `{name}` with args: {args}\n"));
            }
            ChatMessage::ToolResult { name, content, .. } => {
                out.push_str(&format!("Tool result `{name}`: {content}\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_for_summary_includes_each_message_kind() {
        let msgs = vec![
            ChatMessage::User {
                content: "find red shoes".into(),
            },
            ChatMessage::ToolCall {
                id: "t1".into(),
                name: "search".into(),
                args: serde_json::json!({"q": "red shoes"}),
            },
            ChatMessage::ToolResult {
                tool_call_id: "t1".into(),
                name: "search".into(),
                content: "3 results".into(),
            },
            ChatMessage::Assistant {
                content: "found them".into(),
            },
        ];
        let rendered = render_for_summary(&msgs);
        assert!(rendered.contains("User: find red shoes"));
        assert!(rendered.contains("Tool call `search`"));
        assert!(rendered.contains("Tool result `search`: 3 results"));
        assert!(rendered.contains("Assistant: found them"));
    }
}
