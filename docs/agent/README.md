# Arche Agent — Documentation

Building LLM-powered agents with `arche::llm` + `arche::agent`.

| Doc | Read when |
|---|---|
| [architecture.md](architecture.md) | You want the mental model: the three modules, which types live where, and how they compose. Includes a component diagram. |
| [sequence.md](sequence.md) | You want to know what actually happens during a chat request — streaming, tool calling, compaction, error paths. Includes a sequence diagram. |
| [extending.md](extending.md) | You want to plug in a custom LLM backend, custom history compactor, or define your own flow and tools. |

Quick jumps:

- **"I just want to run an agent"** → [extending.md → Getting started](extending.md#getting-started)
- **"How do I add a tool?"** → [extending.md → `AgentFlow`](extending.md#custom-flow-and-tools)
- **"Which type goes where?"** → [architecture.md → Map](architecture.md#map)
- **"What SSE events does the engine emit?"** → [sequence.md](sequence.md)
