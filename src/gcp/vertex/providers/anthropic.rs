use crate::error::AppError;
use crate::gcp::vertex::client::VertexClient;
use crate::gcp::vertex::config::ResolvedAuth;
use crate::gcp::vertex::types::*;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

// --- Wire types ---

#[derive(Serialize)]
struct Request {
    anthropic_version: String,
    max_tokens: u32,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<WireToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct WireMessage {
    role: String,
    content: WireContent,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum WireContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct WireToolDef {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct Response {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct WireUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart {},
    #[serde(rename = "content_block_start")]
    ContentBlockStart {},
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: Delta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {},
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaBody },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Delta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        #[allow(dead_code)]
        partial_json: String,
    },
}

#[derive(Deserialize)]
struct MessageDeltaBody {
    stop_reason: Option<String>,
}

// --- Public API ---

pub(crate) async fn generate(
    client: &VertexClient,
    request: &GenerateRequest,
) -> Result<GenerateResponse, AppError> {
    let url = endpoint(&client.auth, &request.model, false)?;
    let mut req = client.http.post(&url).json(&to_wire(request));

    if let Some(auth) = client.auth_header().await? {
        req = req.header("Authorization", auth);
    }

    let resp = client.send(req).await?;
    let wire: Response = resp
        .json()
        .await
        .map_err(|e| AppError::dependency_failed("vertex-ai", format!("Parse failed: {e}")))?;

    Ok(from_wire(wire))
}

pub(crate) async fn stream_generate(
    client: &VertexClient,
    request: &GenerateRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AppError>> + Send>>, AppError> {
    let url = endpoint(&client.auth, &request.model, true)?;
    let mut wire = to_wire(request);
    wire.stream = Some(true);
    let mut req = client.http.post(&url).json(&wire);

    if let Some(auth) = client.auth_header().await? {
        req = req.header("Authorization", auth);
    }

    let resp = client.send(req).await?;
    Ok(parse_sse(resp))
}

// --- Endpoint ---

fn endpoint(auth: &ResolvedAuth, model: &str, stream: bool) -> Result<String, AppError> {
    let method = if stream {
        "streamRawPredict"
    } else {
        "rawPredict"
    };
    match auth {
        ResolvedAuth::ApiKey { .. } => Err(AppError::internal_error(
            "Anthropic models on Vertex AI require service account auth (VERTEX_PROJECT_ID + GOOGLE_APPLICATION_CREDENTIALS)".into(),
            None,
        )),
        ResolvedAuth::ServiceAccount {
            project_id,
            region,
            ..
        } => Ok(format!(
            "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/anthropic/models/{model}:{method}"
        )),
    }
}

// --- Conversion ---

fn to_wire(req: &GenerateRequest) -> Request {
    let messages = req
        .messages
        .iter()
        .map(|m| WireMessage {
            role: match m.role {
                Role::User => "user".into(),
                Role::Assistant => "assistant".into(),
            },
            content: match m.content.as_slice() {
                [ContentPart::Text(text)] => WireContent::Text(text.clone()),
                parts => WireContent::Blocks(to_blocks(parts)),
            },
        })
        .collect();

    let tools = req.tools.as_ref().map(|defs| {
        defs.iter()
            .map(|t| WireToolDef {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect()
    });

    Request {
        anthropic_version: "vertex-2023-10-16".into(),
        max_tokens: req.max_tokens.unwrap_or(4096),
        messages,
        system: req.system.clone(),
        tools,
        temperature: req.temperature,
        top_p: req.top_p,
        top_k: req.top_k,
        stream: None,
    }
}

fn to_blocks(parts: &[ContentPart]) -> Vec<ContentBlock> {
    parts
        .iter()
        .map(|p| match p {
            ContentPart::Text(text) => ContentBlock::Text { text: text.clone() },
            ContentPart::ToolCall {
                id,
                name,
                arguments,
            } => ContentBlock::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: arguments.clone(),
            },
            ContentPart::ToolResult {
                tool_call_id,
                content,
                ..
            } => ContentBlock::ToolResult {
                tool_use_id: tool_call_id.clone(),
                content: content.to_string(),
            },
        })
        .collect()
}

fn from_wire(resp: Response) -> GenerateResponse {
    let content = resp
        .content
        .into_iter()
        .map(|block| match block {
            ContentBlock::Text { text } => ContentPart::Text(text),
            ContentBlock::ToolUse { id, name, input } => ContentPart::ToolCall {
                id,
                name,
                arguments: input,
            },
            ContentBlock::ToolResult { .. } => ContentPart::Text("[tool_result]".into()),
        })
        .collect();

    let usage = resp.usage.map(|u| Usage {
        input_tokens: Some(u.input_tokens),
        output_tokens: Some(u.output_tokens),
        total_tokens: Some(u.input_tokens + u.output_tokens),
    });

    GenerateResponse {
        content,
        stop_reason: resp.stop_reason,
        usage,
    }
}

// --- SSE parser ---

fn parse_sse(
    resp: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, AppError>> + Send>> {
    let stream = async_stream::stream! {
        let mut byte_stream = futures::StreamExt::fuse(resp.bytes_stream());
        let mut buffer = String::new();

        while let Some(chunk) = futures::StreamExt::next(&mut byte_stream).await {
            let bytes = chunk.map_err(|e| {
                AppError::dependency_failed("vertex-ai", format!("Stream read error: {e}"))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                // Anthropic SSE uses `event: <type>\ndata: <json>` — extract the data line
                let mut data_line = None;
                for line in event_block.lines() {
                    if let Some(d) = line.strip_prefix("data: ") {
                        data_line = Some(d);
                    }
                }

                let Some(data) = data_line else {
                    continue;
                };

                match serde_json::from_str::<StreamEvent>(data) {
                    Ok(event) => match event {
                        StreamEvent::ContentBlockDelta { delta: Delta::TextDelta { text }, .. } => {
                            yield Ok(StreamChunk::Text(text));
                        }
                        StreamEvent::MessageDelta { delta, .. } => {
                            if let Some(reason) = delta.stop_reason {
                                yield Ok(StreamChunk::Done {
                                    finish_reason: reason,
                                });
                            }
                        }
                        StreamEvent::MessageStop => {
                            yield Ok(StreamChunk::Done {
                                finish_reason: "end_turn".into(),
                            });
                        }
                        _ => {}
                    },
                    Err(e) => {
                        tracing::debug!(
                            "Failed to parse Anthropic SSE chunk: {e}, data: {data}"
                        );
                    }
                }
            }
        }
    };

    Box::pin(stream)
}
