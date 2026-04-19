mod compactor;
mod config;
mod engine;
mod stream;
mod types;

pub use compactor::{HistoryCompactor, LlmSummaryCompactor};
pub use config::{AgentConfig, AgentConfigBuilder};
pub use engine::AgentEngine;
pub use stream::to_sse_event;
pub use types::*;

use crate::llm::LlmProvider;
use std::sync::Arc;

pub fn get_agent_engine(provider: impl LlmProvider + 'static, config: AgentConfig) -> AgentEngine {
    AgentEngine::new(Arc::new(provider), config)
}
