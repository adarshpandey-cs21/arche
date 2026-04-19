# Request Sequence

A single chat request end-to-end. See [architecture.md](architecture.md) for the static picture of who-talks-to-whom.

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant H as Handler<br/>(consumer)
    participant S as Storage<br/>(consumer)
    participant E as AgentEngine
    participant K as HistoryCompactor<br/>(optional)
    participant F as AgentFlow<br/>(consumer)
    participant P as LlmProvider<br/>(e.g. VertexClient)
    participant L as Upstream LLM

    C->>H: POST /chat { session_id?, message }
    H->>S: load or create AgentSession
    S-->>H: AgentSession

    H->>E: run(flow, &mut session, message)
    activate E
    E->>E: push ChatMessage::User(message)

    opt session.messages.len() > max_history_messages
        alt compactor configured
            E->>K: compact(&prefix_at_User_boundary)
            K->>P: generate(summary request)
            P->>L: POST generate
            L-->>P: GenerateResponse
            P-->>K: GenerateResponse
            K-->>E: ChatMessage::Assistant("[summary]...")
            E->>E: replace prefix with summary
        else no compactor / compactor errored
            E->>E: drain prefix + strip orphan ToolResult
        end
    end

    loop until no tool call (max_tool_rounds)
        E->>P: stream_generate(llm::GenerateRequest)
        P->>L: POST streamGenerateContent / streamRawPredict
        L-->>P: SSE stream opened

        loop per StreamChunk until Done
            alt Text(delta)
                L-->>P: SSE frame (text delta)
                P-->>E: StreamChunk::Text
                E-->>H: SseEvent::Text
                H-->>C: event: text
            else ToolCall { id, name, arguments }
                L-->>P: SSE frame (tool_use / functionCall)
                P-->>E: StreamChunk::ToolCall
                E-->>H: SseEvent::ToolStatus(Calling)
                H-->>C: event: tool_status
                E->>F: execute_tool(name, args, &session)
                F-->>E: ToolOutput { content, data?, session_metadata? }
                opt output.data present
                    E-->>H: SseEvent::Data
                    H-->>C: event: data
                end
                opt output.session_metadata present
                    E->>E: merge into session.metadata
                end
                E->>E: push ChatMessage::ToolCall + ChatMessage::ToolResult
                E-->>H: SseEvent::ToolStatus(Done)
                H-->>C: event: tool_status
            else Done { finish_reason, usage }
                L-->>P: SSE frame (message_stop / STOP)
                P-->>E: StreamChunk::Done
            end
        end

        alt got_tool_call and rounds < max_tool_rounds
            E->>E: continue — next LLM round with tool result context
        else done or max_tool_rounds exceeded
            E->>E: break
        end
    end

    E->>E: session.last_active = now_rfc3339()
    E-->>H: SseEvent::Done { session_id }
    deactivate E
    H-->>C: event: done

    H->>S: save AgentSession
```

## Error paths

Not drawn in the diagram so the happy path stays readable. All paths still terminate with `SseEvent::Done`, so the handler always sees a clean end-of-stream.

| Failure | Engine emits | Loop behaviour |
|---|---|---|
| `provider.stream_generate()` returns Err | `SseEvent::Error { code: "llm_error", .. }` | outer loop breaks |
| Mid-stream chunk is Err | `SseEvent::Error { code: "stream_error", .. }` | inner chunk loop breaks; outer continues if a tool call was already handled this round |
| `flow.execute_tool()` returns Err | `SseEvent::ToolStatus(Error)` + `SseEvent::Error { code: "tool_error", .. }` | error text is fed back to the LLM as the tool's content so the model can recover; loop continues |
| `HistoryCompactor::compact()` returns Err | (none to the client) | logged at `warn!`; falls through to raw truncation |
| `max_tool_rounds` reached | `SseEvent::Error { code: "max_tool_rounds", .. }` | outer loop breaks |

## What the client sees over the wire

Each `SseEvent` variant maps to a distinct SSE event type (via `to_sse_event`):

| `SseEvent` variant | SSE `event:` | `data:` payload |
|---|---|---|
| `Text { delta }` | `text` | raw string |
| `ToolStatus { tool, status }` | `tool_status` | `{"tool": "<name>", "status": "calling"\|"done"\|"error"}` |
| `Data { type, payload }` | `data` | `{"type": "<type>", "payload": <json>}` |
| `Error { code, message }` | `error` | `{"code": "<code>", "message": "<text>"}` |
| `Done { session_id }` | `done` | `{"session_id": "<id>"}` |

## Session mutations during a run

The engine mutates `session` in place. After the stream completes, the consumer persists it. In order:

1. **User message** pushed at the start.
2. **History compaction** (optional) — replaces a prefix with a single `Assistant` summary message, or raw-truncates.
3. **Per tool call**: pushes `ToolCall` + `ToolResult` (the full round-trip), and optionally merges `ToolOutput.session_metadata` into `session.metadata`.
4. **Assistant text** pushed at the end of each LLM round that produced any text.
5. **`session.last_active`** updated to current RFC3339 timestamp just before `Done`.

Saving after the stream completes gives the consumer a consistent snapshot of the whole turn.
