use crate::error::AppError;
use crate::llm::{self, LlmProvider, StreamChunk};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

use super::compactor::{HistoryCompactor, LlmSummaryCompactor};
use super::config::AgentConfig;
use super::types::*;

pub struct AgentEngine {
    provider: Arc<dyn LlmProvider>,
    compactor: Option<Arc<dyn HistoryCompactor>>,
    config: AgentConfig,
}

impl AgentEngine {
    pub(crate) fn new(provider: Arc<dyn LlmProvider>, config: AgentConfig) -> Self {
        Self {
            provider,
            compactor: None,
            config,
        }
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn with_compactor(mut self, compactor: impl HistoryCompactor + 'static) -> Self {
        self.compactor = Some(Arc::new(compactor));
        self
    }

    /// Shortcut for the built-in LLM-based summarizer that reuses this engine's provider.
    pub fn with_default_summarizer(mut self, model: impl Into<String>) -> Self {
        self.compactor = Some(Arc::new(LlmSummaryCompactor::new(
            Arc::clone(&self.provider),
            model,
        )));
        self
    }

    pub fn run<'a>(
        &'a self,
        flow: &'a dyn AgentFlow,
        session: &'a mut AgentSession,
        message: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<SseEvent, AppError>> + Send + 'a>> {
        let stream = async_stream::stream! {
            session.messages.push(ChatMessage::User {
                content: message.to_string(),
            });

            let tools = flow.tool_definitions();
            let system = flow.system_prompt();
            let model = self.config.model.clone();
            let mut tool_rounds = 0usize;

            loop {
                if let Some(r) = compact_or_truncate(
                    &mut session.messages,
                    self.config.max_history_messages,
                    self.compactor.as_deref(),
                ).await {
                    yield Ok(SseEvent::Data {
                        r#type: "compaction".into(),
                        payload: serde_json::json!({
                            "strategy": r.strategy,
                            "messages_before": r.messages_before,
                            "messages_after": r.messages_after,
                            "summarized": r.summarized,
                            "summary": r.summary,
                            "elapsed_ms": r.elapsed_ms,
                        }),
                    });
                }

                let request = build_llm_request(&model, &session.messages, &tools, &system);

                let chunk_stream = match self.provider.stream_generate(&request).await {
                    Ok(s) => s,
                    Err(e) => {
                        yield Ok(SseEvent::Error {
                            code: "llm_error".into(),
                            message: format!("{e:?}"),
                        });
                        break;
                    }
                };

                let mut full_text = String::new();
                let mut got_tool_call = false;

                futures::pin_mut!(chunk_stream);
                while let Some(chunk) = futures::StreamExt::next(&mut chunk_stream).await {
                    match chunk {
                        Ok(StreamChunk::Text(text)) => {
                            if text.is_empty() {
                                continue;
                            }
                            full_text.push_str(&text);
                            yield Ok(SseEvent::Text { delta: text });
                        }
                        Ok(StreamChunk::ToolCall { id, name, arguments }) => {
                            got_tool_call = true;

                            yield Ok(SseEvent::ToolStatus {
                                tool: name.clone(),
                                status: ToolCallStatus::Calling,
                            });

                            let tool_output = match flow
                                .execute_tool(&name, &arguments, session)
                                .await
                            {
                                Ok(output) => output,
                                Err(e) => {
                                    yield Ok(SseEvent::ToolStatus {
                                        tool: name.clone(),
                                        status: ToolCallStatus::Error,
                                    });
                                    yield Ok(SseEvent::Error {
                                        code: "tool_error".into(),
                                        message: e.to_string(),
                                    });
                                    ToolOutput::text(format!("Error executing {name}: {e}"))
                                }
                            };

                            if let Some(data) = &tool_output.data {
                                yield Ok(SseEvent::Data {
                                    r#type: data.r#type.clone(),
                                    payload: data.payload.clone(),
                                });
                            }

                            if let Some(meta) = &tool_output.session_metadata
                                && let (Some(existing), Some(new)) =
                                    (session.metadata.as_object_mut(), meta.as_object())
                            {
                                for (k, v) in new {
                                    existing.insert(k.clone(), v.clone());
                                }
                            }

                            session.messages.push(ChatMessage::ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                args: arguments,
                            });
                            session.messages.push(ChatMessage::ToolResult {
                                tool_call_id: id,
                                name: name.clone(),
                                content: tool_output.content,
                            });

                            yield Ok(SseEvent::ToolStatus {
                                tool: name,
                                status: ToolCallStatus::Done,
                            });
                        }
                        Ok(StreamChunk::Done { .. }) => {}
                        Err(e) => {
                            yield Ok(SseEvent::Error {
                                code: "stream_error".into(),
                                message: format!("{e:?}"),
                            });
                            break;
                        }
                    }
                }

                if !full_text.is_empty() {
                    session.messages.push(ChatMessage::Assistant {
                        content: full_text,
                    });
                }

                if got_tool_call {
                    tool_rounds += 1;
                    if tool_rounds >= self.config.max_tool_rounds {
                        yield Ok(SseEvent::Error {
                            code: "max_tool_rounds".into(),
                            message: "Maximum tool calling rounds exceeded".into(),
                        });
                        break;
                    }
                    continue;
                }

                break;
            }

            session.last_active = now_rfc3339();
            yield Ok(SseEvent::Done {
                session_id: session.id.clone(),
            });
        };

        Box::pin(stream)
    }
}

fn session_to_llm_messages(messages: &[ChatMessage]) -> Vec<llm::Message> {
    messages
        .iter()
        .map(|msg| match msg {
            ChatMessage::User { content } => llm::Message::user(content),
            ChatMessage::Assistant { content } => llm::Message::assistant(content),
            ChatMessage::ToolCall { id, name, args } => {
                llm::Message::tool_call(id, name, args.clone())
            }
            ChatMessage::ToolResult {
                tool_call_id,
                name,
                content,
            } => llm::Message::tool_result(
                tool_call_id,
                name,
                serde_json::Value::String(content.clone()),
            ),
        })
        .collect()
}

fn build_llm_request(
    model: &str,
    messages: &[ChatMessage],
    tools: &[llm::ToolDefinition],
    system: &str,
) -> llm::GenerateRequest {
    let mut req = llm::GenerateRequest::new(model, session_to_llm_messages(messages))
        .with_tools(tools.to_vec());

    if !system.is_empty() {
        req = req.with_system(system);
    }

    req
}

/// Bring `messages.len()` down to `max`. If a compactor is configured, split at
/// the nearest `User` boundary ≥ the ideal cut and replace the head with the
/// compactor's summary (so tool-call/result pairs stay intact). Otherwise, or
/// on compactor failure, drop the head outright, then strip any leading
/// `ToolResult` so the model never sees an orphan.
/// Result of a compaction / truncation pass. Returned so callers (e.g. the
/// engine) can surface it as an observable event.
pub struct CompactionResult {
    pub strategy: &'static str, // "summarize" or "truncate"
    pub messages_before: usize,
    pub messages_after: usize,
    pub summarized: usize,
    pub summary: Option<String>, // present only when strategy == "summarize"
    pub elapsed_ms: u64,
}

async fn compact_or_truncate(
    messages: &mut Vec<ChatMessage>,
    max: usize,
    compactor: Option<&dyn HistoryCompactor>,
) -> Option<CompactionResult> {
    if max == 0 || messages.len() <= max {
        return None;
    }

    let before_len = messages.len();
    let ideal = messages.len() - max;
    let start = std::time::Instant::now();

    if let Some(c) = compactor
        && let Some(split) = find_user_split(messages, ideal)
        && split > 0
    {
        tracing::info!(
            before_len,
            max_history = max,
            compacting = split,
            "History exceeds max; running LLM compactor"
        );
        let prefix: Vec<ChatMessage> = messages.drain(..split).collect();
        match c.compact(&prefix).await {
            Ok(summary_msg) => {
                let summary_text = match &summary_msg {
                    ChatMessage::Assistant { content } => content.clone(),
                    _ => String::new(),
                };
                messages.insert(0, summary_msg);
                let after_len = messages.len();
                return Some(CompactionResult {
                    strategy: "summarize",
                    messages_before: before_len,
                    messages_after: after_len,
                    summarized: split,
                    summary: Some(summary_text),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "history compactor failed; falling back to raw truncation",
                );
                strip_orphan_tool_results_head(messages);
                let after_len = messages.len();
                return Some(CompactionResult {
                    strategy: "truncate",
                    messages_before: before_len,
                    messages_after: after_len,
                    summarized: split,
                    summary: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // No compactor, or no valid split: raw truncate.
    messages.drain(..ideal);
    strip_orphan_tool_results_head(messages);
    let after_len = messages.len();
    Some(CompactionResult {
        strategy: "truncate",
        messages_before: before_len,
        messages_after: after_len,
        summarized: ideal,
        summary: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

fn find_user_split(messages: &[ChatMessage], ideal: usize) -> Option<usize> {
    (ideal..messages.len()).find(|&i| matches!(&messages[i], ChatMessage::User { .. }))
}

fn strip_orphan_tool_results_head(messages: &mut Vec<ChatMessage>) {
    while matches!(messages.first(), Some(ChatMessage::ToolResult { .. })) {
        messages.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn user(c: &str) -> ChatMessage {
        ChatMessage::User { content: c.into() }
    }
    fn assistant(c: &str) -> ChatMessage {
        ChatMessage::Assistant { content: c.into() }
    }
    fn tc(id: &str) -> ChatMessage {
        ChatMessage::ToolCall {
            id: id.into(),
            name: "t".into(),
            args: serde_json::json!({}),
        }
    }
    fn tr(id: &str) -> ChatMessage {
        ChatMessage::ToolResult {
            tool_call_id: id.into(),
            name: "t".into(),
            content: "ok".into(),
        }
    }

    struct StubCompactor {
        calls: Mutex<Vec<Vec<ChatMessage>>>,
        result: Result<ChatMessage, AppError>,
    }

    impl StubCompactor {
        fn ok() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                result: Ok(ChatMessage::Assistant {
                    content: "[summary]".into(),
                }),
            }
        }
        fn err() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                result: Err(AppError::internal_error("boom".into(), None)),
            }
        }
    }

    impl HistoryCompactor for StubCompactor {
        fn compact<'a>(
            &'a self,
            messages: &'a [ChatMessage],
        ) -> Pin<Box<dyn futures::Future<Output = Result<ChatMessage, AppError>> + Send + 'a>>
        {
            self.calls.lock().unwrap().push(messages.to_vec());
            let result = match &self.result {
                Ok(m) => Ok(m.clone()),
                Err(e) => Err(AppError::internal_error(e.to_string(), None)),
            };
            Box::pin(async move { result })
        }
    }

    #[tokio::test]
    async fn noop_when_under_limit() {
        let mut msgs = vec![user("a"), assistant("b")];
        compact_or_truncate(&mut msgs, 10, None).await;
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn raw_truncate_when_no_compactor() {
        let mut msgs = vec![user("a"), assistant("b"), user("c"), assistant("d")];
        compact_or_truncate(&mut msgs, 2, None).await;
        assert_eq!(msgs.len(), 2);
        assert!(matches!(&msgs[0], ChatMessage::User { content } if content == "c"));
    }

    #[tokio::test]
    async fn raw_truncate_strips_orphan_tool_result() {
        let mut msgs = vec![user("a"), tc("1"), tr("1"), assistant("b")];
        compact_or_truncate(&mut msgs, 2, None).await;
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0], ChatMessage::Assistant { .. }));
    }

    #[tokio::test]
    async fn compactor_replaces_prefix_with_summary() {
        let mut msgs = vec![
            user("first"),
            assistant("r1"),
            tc("1"),
            tr("1"),
            assistant("r2"),
            user("second"),
            assistant("r3"),
        ];
        let c = StubCompactor::ok();
        compact_or_truncate(&mut msgs, 2, Some(&c)).await;

        // Prefix is [first, r1, tc, tr, r2] (split at "second" = index 5).
        // After replacement: [summary, second, r3].
        assert_eq!(msgs.len(), 3);
        assert!(matches!(&msgs[0], ChatMessage::Assistant { content } if content == "[summary]"));
        assert!(matches!(&msgs[1], ChatMessage::User { content } if content == "second"));

        let calls = c.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].len(), 5);
    }

    #[tokio::test]
    async fn compactor_error_falls_back_to_truncation() {
        let mut msgs = vec![
            user("first"),
            assistant("r1"),
            user("second"),
            assistant("r2"),
        ];
        let c = StubCompactor::err();
        compact_or_truncate(&mut msgs, 2, Some(&c)).await;

        // Prefix drained, no summary inserted.
        assert_eq!(msgs.len(), 2);
        assert!(matches!(&msgs[0], ChatMessage::User { content } if content == "second"));
    }

    #[tokio::test]
    async fn compactor_preserves_tool_pair_via_user_boundary() {
        // Ideal cut at index 2 (len=5, max=3) lands inside a ToolCall/ToolResult pair.
        // Split must extend to next User at index 3.
        let mut msgs = vec![
            user("start"),
            tc("1"),
            tr("1"),
            user("mid"),
            assistant("ok"),
        ];
        let c = StubCompactor::ok();
        compact_or_truncate(&mut msgs, 3, Some(&c)).await;

        assert_eq!(msgs.len(), 3); // summary, user(mid), assistant(ok)
        assert!(matches!(&msgs[0], ChatMessage::Assistant { content } if content == "[summary]"));
        assert!(matches!(&msgs[1], ChatMessage::User { content } if content == "mid"));

        let calls = c.calls.lock().unwrap();
        assert_eq!(calls[0].len(), 3); // Prefix contains full tool pair.
    }

    #[test]
    fn session_to_llm_messages_maps_each_variant() {
        let msgs = vec![user("hi"), tc("call_1"), tr("call_1"), assistant("there")];
        let mapped = session_to_llm_messages(&msgs);
        assert_eq!(mapped.len(), 4);
        assert_eq!(mapped[0].role, llm::Role::User);
        assert_eq!(mapped[1].role, llm::Role::Assistant);
        assert_eq!(mapped[2].role, llm::Role::User);
        assert_eq!(mapped[3].role, llm::Role::Assistant);
    }
}
