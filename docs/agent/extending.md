# Extending

Four plug points, in order of likelihood:

| You want to | Plug into |
|---|---|
| Define agent behaviour (prompts, tools) | `impl AgentFlow` |
| Swap the LLM backend (OpenAI, Bedrock, Ollama, local) | `impl LlmProvider` |
| Replace default summarization (vector recall, server-side memory) | `impl HistoryCompactor` |
| Surface custom UI events to the client | Return `ToolOutput::text(..).data(type, payload)` |

Nothing in `arche::agent` changes when you extend any of these.

## Getting started

Minimum viable agent with the built-in Vertex + Gemini:

```rust
use arche::agent::{get_agent_engine, AgentConfig, AgentFlow, AgentSession, ToolOutput, to_sse_event};
use arche::error::AppError;
use arche::gcp::vertex::{get_vertex_client, VertexProvider};
use arche::llm::ToolDefinition;

struct MyFlow;

impl AgentFlow for MyFlow {
    fn system_prompt(&self) -> String {
        "You are a helpful assistant.".into()
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![]
    }

    fn execute_tool<'a>(
        &'a self,
        _name: &'a str,
        _args: &'a serde_json::Value,
        _session: &'a AgentSession,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolOutput, AppError>> + Send + 'a>> {
        Box::pin(async move { Ok(ToolOutput::text("")) })
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let client = get_vertex_client(VertexProvider::Gemini, None).await?;
    let config = AgentConfig::builder("gemini-2.0-flash").build()?;
    let engine = get_agent_engine(client, config);

    let mut session = AgentSession::new("sess-1", "demo");
    let stream = engine.run(&MyFlow, &mut session, "Hi!");

    use futures::StreamExt;
    futures::pin_mut!(stream);
    while let Some(event) = stream.next().await {
        if let Ok(e) = event {
            let sse = to_sse_event(e);
            println!("{sse:?}");
        }
    }
    Ok(())
}
```

## Custom flow and tools

`AgentFlow` captures your domain: system prompt, tool schemas, tool executors. The flow can hold DB pools, API clients, cached data — anything. It has to be `Send + Sync`.

```rust
use arche::agent::{AgentFlow, AgentSession, ToolOutput};
use arche::error::AppError;
use arche::llm::{ToolDefinition, ParameterSchema};
use std::pin::Pin;
use std::future::Future;

struct ShoppingFlow {
    db: sqlx::PgPool,
}

impl AgentFlow for ShoppingFlow {
    fn system_prompt(&self) -> String {
        "You help shoppers find products. Use search_catalog when they ask for something.".into()
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::new(
                "search_catalog",
                "Search the product catalog by natural-language query.",
            )
            .with_parameters(
                ParameterSchema::object()
                    .with_property(
                        "query",
                        ParameterSchema::string("Natural-language search query"),
                    )
                    .with_property(
                        "limit",
                        ParameterSchema::integer("Max results (default 5)"),
                    )
                    .with_required(["query"]),
            ),
        ]
    }

    fn execute_tool<'a>(
        &'a self,
        name: &'a str,
        args: &'a serde_json::Value,
        _session: &'a AgentSession,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput, AppError>> + Send + 'a>> {
        Box::pin(async move {
            match name {
                "search_catalog" => {
                    let query = args["query"].as_str().unwrap_or("");
                    // ... run sqlx query ...
                    let products = serde_json::json!([{"id": "p1", "name": "Red shoes"}]);

                    Ok(ToolOutput::text(format!("Found 1 match for {query}."))
                        .data("product_list", products))
                }
                _ => Ok(ToolOutput::text(format!("Unknown tool: {name}"))),
            }
        })
    }
}
```

Key points:

- **`ToolDefinition` is typed** — use the `ParameterSchema` builders; don't hand-write JSON Schema.
- **`ToolOutput::text(..).data(..)` is dual-output** — `text` is what the LLM sees and reasons over; `data` reaches the client directly via `SseEvent::Data` for custom UI rendering. Use text-only for "facts for the model" and add data for "render this card in the UI."
- **Session is read-only to tools.** Tools see `&AgentSession`, can read metadata/history for context, but mutation goes through `ToolOutput.session_metadata` which the engine merges.

## Custom LLM backend

Any type implementing `arche::llm::LlmProvider` drops into `get_agent_engine`. Example: OpenAI Chat Completions.

```rust
use arche::error::AppError;
use arche::llm::{GenerateRequest, GenerateResponse, LlmProvider, LlmStream, StreamChunk};
use std::future::Future;
use std::pin::Pin;

pub struct OpenAiClient {
    http: reqwest::Client,
    api_key: String,
}

impl LlmProvider for OpenAiClient {
    fn generate<'a>(
        &'a self,
        request: &'a GenerateRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GenerateResponse, AppError>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Convert `request` → OpenAI's JSON body
            // 2. POST https://api.openai.com/v1/chat/completions
            // 3. Convert response → canonical GenerateResponse
            todo!()
        })
    }

    fn stream_generate<'a>(
        &'a self,
        request: &'a GenerateRequest,
    ) -> Pin<Box<dyn Future<Output = Result<LlmStream, AppError>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Convert `request` with stream=true
            // 2. POST, parse OpenAI SSE
            // 3. Yield StreamChunk::{Text, ToolCall, Done} as frames arrive
            todo!()
        })
    }
}
```

From the engine's perspective this is identical to `VertexClient`:

```rust
let engine = get_agent_engine(OpenAiClient { .. }, config);
```

## Custom history compactor

The default `LlmSummaryCompactor` makes an extra LLM call every time history overflows. If that's too expensive, or you want semantic recall, implement `HistoryCompactor` yourself.

```rust
use arche::agent::{ChatMessage, HistoryCompactor};
use arche::error::AppError;
use std::future::Future;
use std::pin::Pin;

struct VectorMemoryCompactor {
    // e.g. a qdrant client
}

impl HistoryCompactor for VectorMemoryCompactor {
    fn compact<'a>(
        &'a self,
        messages: &'a [ChatMessage],
    ) -> Pin<Box<dyn Future<Output = Result<ChatMessage, AppError>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Embed messages, upsert to your vector store keyed by session id.
            // 2. Return a stub Assistant message so the model knows prior history exists.
            Ok(ChatMessage::Assistant {
                content: "[prior turns stored in long-term memory]".into(),
            })
        })
    }
}
```

Attach it to the engine:

```rust
let engine = get_agent_engine(client, config)
    .with_compactor(VectorMemoryCompactor { /* ... */ });
```

Or use the built-in summarizer without writing your own:

```rust
let engine = get_agent_engine(client, config)
    .with_default_summarizer("gemini-2.0-flash-lite"); // cheaper model for summaries
```

## Surfacing custom client events

Anything interesting a tool produces — UI cards, inline citations, product thumbnails, progress indicators — can reach the client without going through the LLM's text channel.

```rust
Ok(ToolOutput::text("Found 3 matches.")
    .data("product_card_list", serde_json::json!([
        { "id": "p1", "name": "Red shoes", "image": "..." },
        { "id": "p2", "name": "Blue shoes", "image": "..." },
        { "id": "p3", "name": "Green shoes", "image": "..." },
    ])))
```

Engine emits `SseEvent::Data { type: "product_card_list", payload: [...] }`. Client JS dispatches on `event.type` and renders the appropriate component.

The LLM only sees `"Found 3 matches."` — it doesn't have to hallucinate JSON to describe the cards.
