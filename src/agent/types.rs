use crate::error::AppError;
use crate::llm::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub trait AgentFlow: Send + Sync {
    fn system_prompt(&self) -> String;

    fn tool_definitions(&self) -> Vec<ToolDefinition>;

    fn execute_tool<'a>(
        &'a self,
        name: &'a str,
        args: &'a serde_json::Value,
        session: &'a AgentSession,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput, AppError>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: String,
    pub data: Option<DataEvent>,
    pub session_metadata: Option<serde_json::Value>,
}

impl ToolOutput {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            data: None,
            session_metadata: None,
        }
    }

    pub fn data(mut self, data_type: impl Into<String>, payload: serde_json::Value) -> Self {
        self.data = Some(DataEvent {
            r#type: data_type.into(),
            payload,
        });
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.session_metadata = Some(metadata);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEvent {
    pub r#type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum SseEvent {
    Text {
        delta: String,
    },
    ToolStatus {
        tool: String,
        status: ToolCallStatus,
        label: Option<String>,
    },
    Data {
        r#type: String,
        payload: serde_json::Value,
    },
    Error {
        code: String,
        message: String,
    },
    Done {
        session_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Calling,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub flow: String,
    pub messages: Vec<ChatMessage>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub last_active: String,
}

impl AgentSession {
    pub fn new(id: impl Into<String>, flow: impl Into<String>) -> Self {
        let now = now_rfc3339();
        Self {
            id: id.into(),
            flow: flow.into(),
            messages: Vec::new(),
            metadata: serde_json::json!({}),
            created_at: now.clone(),
            last_active: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatMessage {
    User {
        content: String,
    },
    Assistant {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
        /// Used by Gemini 2.5+ thinking models. Other providers serialize this as absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thought_signature: Option<String>,
    },
    ToolResult {
        tool_call_id: String,
        name: String,
        content: String,
    },
}

pub(crate) fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "RFC3339 format unexpectedly failed; using epoch");
            "1970-01-01T00:00:00Z".into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_enum_serializes_tagged() {
        let msg = ChatMessage::User {
            content: "hi".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["kind"], "user");
        assert_eq!(json["content"], "hi");

        let tool = ChatMessage::ToolCall {
            id: "t1".into(),
            name: "search".into(),
            args: serde_json::json!({"q": "x"}),
            thought_signature: None,
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["kind"], "tool_call");
        assert_eq!(json["id"], "t1");
    }

    #[test]
    fn chat_message_round_trips() {
        let original = ChatMessage::ToolResult {
            tool_call_id: "t1".into(),
            name: "search".into(),
            content: "ok".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        match back {
            ChatMessage::ToolResult {
                tool_call_id,
                name,
                content,
            } => {
                assert_eq!(tool_call_id, "t1");
                assert_eq!(name, "search");
                assert_eq!(content, "ok");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn agent_session_new_has_valid_timestamps_and_object_metadata() {
        let s = AgentSession::new("abc", "shopping");
        assert_eq!(s.id, "abc");
        assert!(!s.created_at.is_empty());
        assert_eq!(s.created_at, s.last_active);
        assert!(s.metadata.is_object());
    }
}
