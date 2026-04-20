use axum::response::sse;

use super::types::{SseEvent, ToolCallStatus};

pub fn to_sse_event(event: SseEvent) -> sse::Event {
    match event {
        SseEvent::Text { delta } => sse::Event::default().event("text").data(delta),

        SseEvent::ToolStatus {
            tool,
            status,
            label,
        } => {
            let status_str = match status {
                ToolCallStatus::Calling => "calling",
                ToolCallStatus::Done => "done",
                ToolCallStatus::Error => "error",
            };
            let mut body = serde_json::json!({ "tool": tool, "status": status_str });
            if let Some(l) = label {
                body["label"] = serde_json::Value::String(l);
            }
            sse::Event::default()
                .event("tool_status")
                .data(body.to_string())
        }

        SseEvent::Data { r#type, payload } => sse::Event::default()
            .event("data")
            .data(serde_json::json!({ "type": r#type, "payload": payload }).to_string()),

        SseEvent::Error { code, message } => sse::Event::default()
            .event("error")
            .data(serde_json::json!({ "code": code, "message": message }).to_string()),

        SseEvent::Done { session_id } => sse::Event::default()
            .event("done")
            .data(serde_json::json!({ "session_id": session_id }).to_string()),
    }
}
