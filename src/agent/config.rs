use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: String,
    pub max_tool_rounds: usize,
    pub max_history_messages: usize,
}

impl AgentConfig {
    pub fn builder(model: impl Into<String>) -> AgentConfigBuilder {
        AgentConfigBuilder {
            model: model.into(),
            max_tool_rounds: None,
            max_history_messages: None,
        }
    }
}

pub struct AgentConfigBuilder {
    model: String,
    max_tool_rounds: Option<usize>,
    max_history_messages: Option<usize>,
}

impl AgentConfigBuilder {
    pub fn max_tool_rounds(mut self, val: usize) -> Self {
        self.max_tool_rounds = Some(val);
        self
    }

    pub fn max_history_messages(mut self, val: usize) -> Self {
        self.max_history_messages = Some(val);
        self
    }

    pub fn build(self) -> Result<AgentConfig, AppError> {
        if self.model.trim().is_empty() {
            return Err(AppError::bad_request(
                None,
                Some("AgentConfig.model must be non-empty".into()),
                None,
            ));
        }
        Ok(AgentConfig {
            model: self.model,
            max_tool_rounds: self.max_tool_rounds.unwrap_or(5),
            max_history_messages: self.max_history_messages.unwrap_or(50),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_applied_on_minimal_build() {
        let c = AgentConfig::builder("gemini-2.0-flash").build().unwrap();
        assert_eq!(c.max_tool_rounds, 5);
        assert_eq!(c.max_history_messages, 50);
    }

    #[test]
    fn overrides_take_effect() {
        let c = AgentConfig::builder("m")
            .max_tool_rounds(10)
            .max_history_messages(100)
            .build()
            .unwrap();
        assert_eq!(c.max_tool_rounds, 10);
        assert_eq!(c.max_history_messages, 100);
    }

    #[test]
    fn empty_model_returns_error() {
        let err = AgentConfig::builder("   ").build().unwrap_err();
        assert!(matches!(err, AppError::BadRequest { .. }));
    }
}
