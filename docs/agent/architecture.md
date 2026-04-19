# Architecture

Three modules, layered bottom-up:

1. **`arche::llm`** — canonical types + the `LlmProvider` trait. No I/O.
2. **`arche::gcp::vertex`** — one `LlmProvider` implementation (Gemini + Anthropic via Vertex).
3. **`arche::agent`** — orchestration loop (engine, session, tool calling, SSE). Depends only on `arche::llm`.

Consumers plug in their own `AgentFlow` (domain logic) and wire the engine into an HTTP handler.

## Map

Hover any node for a one-line explanation (GitHub, VS Code, and mermaid.live all render the tooltips).

```mermaid
flowchart LR
    Consumer["Consumer code<br/><i>impl AgentFlow, HTTP handler</i>"]

    subgraph LLM["arche::llm — canonical abstraction"]
        direction TB
        LP(("LlmProvider<br/>trait"))
        Req["GenerateRequest"]
        Resp["GenerateResponse"]
        Msg["Message"]
        SC["StreamChunk"]
        TD["ToolDefinition"]
        PS["ParameterSchema"]
        Usage["Usage"]
    end

    subgraph VTX["arche::gcp::vertex — built-in backend"]
        direction TB
        VC["VertexClient<br/><i>impl LlmProvider</i>"]
        VP["VertexProvider<br/><i>Gemini | Anthropic</i>"]
        GVC["get_vertex_client()"]
    end

    subgraph AG["arche::agent — orchestrator"]
        direction TB
        AE["AgentEngine"]
        AC["AgentConfig"]
        AF(("AgentFlow<br/>trait"))
        AS["AgentSession"]
        CM["ChatMessage"]
        TO["ToolOutput"]
        SE["SseEvent"]
        HC(("HistoryCompactor<br/>trait"))
        LSC["LlmSummaryCompactor"]
        GAE["get_agent_engine()"]
        TSE["to_sse_event()"]
    end

    %% Trait implementations
    VC -. implements .-> LP
    LSC -. implements .-> HC
    Consumer -. implements .-> AF

    %% Runtime composition
    LSC --> LP
    AE -->|Arc&lt;dyn&gt;| LP
    AE -->|Option&lt;Arc&lt;dyn&gt;&gt;| HC
    AE -->|reads| AC
    AE -->|mutates| AS
    AS -->|Vec&lt;_&gt;| CM
    AE -->|yields Stream| SE
    AF -->|returns| TD
    AF -->|returns| TO

    %% Factories / helpers
    Consumer --> GVC
    Consumer --> GAE
    Consumer --> TSE
    GVC -->|returns| VC
    GAE -->|builds| AE

    %% Tooltips (hover)
    click LP href "#llmprovider" "Abstraction over any LLM backend. generate() and stream_generate() on a canonical GenerateRequest. Engine calls it via Arc<dyn LlmProvider>."
    click Req href "#generaterequest" "Canonical LLM request: model, messages, system prompt, max_tokens, temperature, top_p, top_k, tools."
    click Resp href "#generateresponse" "Canonical LLM response: content parts, stop_reason, Option<Usage>. text() and tool_calls() accessors."
    click Msg href "#message" "Conversation turn: role + Vec<ContentPart>. Helpers: user(), assistant(), tool_call(id,name,args), tool_result(id,name,content)."
    click SC href "#streamchunk" "One streaming chunk from a provider: Text(String) | ToolCall{id,name,arguments} | Done{finish_reason, Option<Usage>}."
    click TD href "#tooldefinition" "Typed tool description: name, description, ParameterSchema. Serializes to JSON Schema."
    click PS href "#parameterschema" "JSON Schema subset with IndexMap-backed properties (insertion order preserved in output). Builders: object/string/integer/number/boolean/array/string_enum."
    click Usage href "#usage" "Token accounting: input_tokens, output_tokens, total_tokens (all Optional)."

    click VC href "#vertexclient" "Built-in LlmProvider impl. Wraps Vertex AI. Captures VertexProvider (Gemini or Anthropic) at construction; model is per-request."
    click VP href "#vertexprovider" "Enum Gemini | Anthropic — which API family the VertexClient talks to. Chosen at get_vertex_client()."
    click GVC href "#get_vertex_client" "async factory: get_vertex_client(provider, Option<VertexConfig>) → Result<VertexClient, AppError>."

    click AE href "#agentengine" "Orchestration loop. Holds Arc<dyn LlmProvider>, optional Arc<dyn HistoryCompactor>, AgentConfig. run(flow, &mut session, msg) returns Stream<SseEvent>."
    click AC href "#agentconfig" "model (required), max_tool_rounds (default 5), max_history_messages (default 50). Built via .builder(model).build() → Result<AgentConfig, AppError>."
    click AF href "#agentflow" "Consumer trait. system_prompt() → String, tool_definitions() → Vec<ToolDefinition>, execute_tool(name, args, &AgentSession) → Future<ToolOutput>."
    click AS href "#agentsession" "Serializable session state: id, flow, messages, metadata, created_at, last_active. Consumer owns persistence."
    click CM href "#chatmessage" "Tagged enum for session: User{content} | Assistant{content} | ToolCall{id,name,args} | ToolResult{tool_call_id,name,content}."
    click TO href "#tooloutput" "Dual-output from a tool: content (fed back to LLM) + data (forwarded to client via SSE) + session_metadata (merged into AgentSession.metadata). Builder: text(c).data(type,payload).metadata(v)."
    click SE href "#sseevent" "Text{delta} | ToolStatus{tool,status} | Data{type,payload} | Error{code,message} | Done{session_id}. Engine emits; to_sse_event converts to axum."
    click HC href "#historycompactor" "Trait: compact(&[ChatMessage]) → Future<ChatMessage>. Called when session length exceeds max_history_messages."
    click LSC href "#llmsummarycompactor" "Default compactor. Uses Arc<dyn LlmProvider> + a cheap model to produce a third-person summary message. with_prompt() and with_max_tokens() overrides available."
    click GAE href "#get_agent_engine" "Factory: get_agent_engine(provider, config) → AgentEngine. Chain .with_compactor(c) or .with_default_summarizer(model)."
    click TSE href "#to_sse_event" "Converts SseEvent → axum::response::sse::Event for HTTP streaming. One function, no state."

    click Consumer href "#consumer" "The service integrating arche: an impl AgentFlow (business logic) + an HTTP handler that creates the engine, loads/saves AgentSession, and streams SseEvents."

    classDef trait fill:#f4e9ff,stroke:#8858c4,color:#333;
    classDef builtin fill:#e9f4ff,stroke:#3c78b8,color:#333;
    classDef consumer fill:#fff4e5,stroke:#c48e3c,color:#333;
    class LP,AF,HC trait
    class VC,LSC,AE,GAE,GVC,TSE builtin
    class Consumer consumer
```

### Legend

- **Oval nodes (`(( ))`)** — traits. Interface boundaries consumers can plug into.
- **Purple-tinted** — trait definitions.
- **Blue-tinted** — concrete built-in code (types, factories, helpers).
- **Orange-tinted** — consumer code (out of this crate).

## Why these three modules

**Why `llm` is separate from `agent`**: the canonical types are valuable on their own. A consumer who just needs "call an LLM" can use `arche::llm` + `arche::gcp::vertex` without touching the agent machinery. The agent module then layers on top without being tangled with backend concerns.

**Why `vertex` lives in `gcp`**: it's a specific backend provider. Future backends (OpenAI, Bedrock, Ollama) belong in their own modules alongside, each implementing `LlmProvider`.

**Why the engine doesn't own a `VertexClient`**: it owns `Arc<dyn LlmProvider>`. Swapping backends costs one line at the call site; zero lines inside `arche::agent`.

## Next

- [sequence.md](sequence.md) — what a request actually looks like at runtime.
- [extending.md](extending.md) — how to write your own `AgentFlow`, swap the LLM backend, or replace the history compactor.
