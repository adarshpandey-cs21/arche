use crate::error::AppError;
use crate::gcp::vertex::client::VertexClient;
use crate::gcp::vertex::config::ResolvedAuth;
use crate::gcp::vertex::types::*;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

// --- Wire types ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenConfig>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Part {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FnCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FnResponse,
    },
}

#[derive(Serialize, Deserialize)]
struct FnCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct FnResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Tool {
    function_declarations: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    candidates: Option<Vec<Candidate>>,
    usage_metadata: Option<UsageMeta>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<Content>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMeta {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

// --- Public API ---

pub(crate) async fn generate(
    client: &VertexClient,
    request: &GenerateRequest,
) -> Result<GenerateResponse, AppError> {
    let url = endpoint(&client.auth, &request.model, false);
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
    let base = endpoint(&client.auth, &request.model, true);
    let sep = if base.contains('?') { '&' } else { '?' };
    let url = format!("{base}{sep}alt=sse");
    let mut req = client.http.post(&url).json(&to_wire(request));

    if let Some(auth) = client.auth_header().await? {
        req = req.header("Authorization", auth);
    }

    let resp = client.send(req).await?;
    Ok(parse_sse(resp))
}

// --- Endpoint ---

fn endpoint(auth: &ResolvedAuth, model: &str, stream: bool) -> String {
    let method = if stream {
        "streamGenerateContent"
    } else {
        "generateContent"
    };
    match auth {
        ResolvedAuth::ApiKey { api_key } => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:{method}?key={api_key}"
        ),
        ResolvedAuth::ServiceAccount {
            project_id, region, ..
        } => format!(
            "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/google/models/{model}:{method}"
        ),
    }
}

// --- Conversion ---

fn to_wire(req: &GenerateRequest) -> Request {
    let contents = req
        .messages
        .iter()
        .map(|m| Content {
            role: match m.role {
                Role::User => "user".into(),
                Role::Assistant => "model".into(),
            },
            parts: m
                .content
                .iter()
                .map(|p| match p {
                    ContentPart::Text(text) => Part::Text { text: text.clone() },
                    ContentPart::ToolCall {
                        name, arguments, ..
                    } => Part::FunctionCall {
                        function_call: FnCall {
                            name: name.clone(),
                            args: arguments.clone(),
                        },
                    },
                    ContentPart::ToolResult { name, content, .. } => Part::FunctionResponse {
                        function_response: FnResponse {
                            name: name.clone(),
                            response: content.clone(),
                        },
                    },
                })
                .collect(),
        })
        .collect();

    let system_instruction = req.system.as_ref().map(|s| Content {
        role: "user".into(),
        parts: vec![Part::Text { text: s.clone() }],
    });

    let generation_config = if req.max_tokens.is_some()
        || req.temperature.is_some()
        || req.top_p.is_some()
        || req.top_k.is_some()
    {
        Some(GenConfig {
            max_output_tokens: req.max_tokens,
            temperature: req.temperature,
            top_p: req.top_p,
            top_k: req.top_k,
        })
    } else {
        None
    };

    let tools = req.tools.as_ref().map(|defs| {
        vec![Tool {
            function_declarations: defs
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    })
                })
                .collect(),
        }]
    });

    Request {
        contents,
        system_instruction,
        generation_config,
        tools,
    }
}

fn from_wire(resp: Response) -> GenerateResponse {
    let mut content = Vec::new();
    let mut stop_reason = None;

    if let Some(candidates) = &resp.candidates
        && let Some(candidate) = candidates.first()
    {
        stop_reason = candidate.finish_reason.clone();
        if let Some(c) = &candidate.content {
            for part in &c.parts {
                match part {
                    Part::Text { text } => {
                        content.push(ContentPart::Text(text.clone()));
                    }
                    Part::FunctionCall { function_call } => {
                        content.push(ContentPart::ToolCall {
                            id: String::new(),
                            name: function_call.name.clone(),
                            arguments: function_call.args.clone(),
                        });
                    }
                    Part::FunctionResponse { .. } => {}
                }
            }
        }
    }

    let usage = resp.usage_metadata.map(|u| Usage {
        input_tokens: u.prompt_token_count,
        output_tokens: u.candidates_token_count,
        total_tokens: u.total_token_count,
    });

    GenerateResponse {
        content,
        stop_reason,
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
                let event = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                if let Some(data) = event.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        yield Ok(StreamChunk::Done {
                            finish_reason: "STOP".into(),
                        });
                        continue;
                    }

                    match serde_json::from_str::<Response>(data) {
                        Ok(response) => {
                            if let Some(candidates) = &response.candidates {
                                for candidate in candidates {
                                    if let Some(content) = &candidate.content {
                                        for part in &content.parts {
                                            if let Part::Text { text } = part {
                                                yield Ok(StreamChunk::Text(text.clone()));
                                            }
                                        }
                                    }
                                    if let Some(reason) = &candidate.finish_reason
                                        && (reason == "STOP" || reason == "MAX_TOKENS")
                                    {
                                        yield Ok(StreamChunk::Done {
                                            finish_reason: reason.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Failed to parse Gemini SSE chunk: {e}, data: {data}");
                        }
                    }
                }
            }
        }
    };

    Box::pin(stream)
}
