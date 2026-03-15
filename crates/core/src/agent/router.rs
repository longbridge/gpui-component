use tokio_util::sync::CancellationToken;

use crate::llm::{ChatRequest, LlmProvider, Message, ProviderConfig, Role};

use super::types::DynAgent;

/// The result of intent routing — which agent should handle the request.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub agent_id: String,
}

/// Errors that can occur during intent routing.
#[derive(Debug)]
pub enum RouterError {
    /// The LLM call itself failed.
    LlmError(String),
    /// We got a response but could not parse an agent id from it.
    ParseFailed(String),
    /// The routing was cancelled.
    Cancelled,
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterError::LlmError(msg) => write!(f, "LLM routing error: {}", msg),
            RouterError::ParseFailed(msg) => write!(f, "Failed to parse routing response: {}", msg),
            RouterError::Cancelled => write!(f, "Routing cancelled"),
        }
    }
}

impl std::error::Error for RouterError {}

/// Prompt-based intent router.
///
/// Asks the LLM to pick the best agent from a list of descriptors.
pub struct IntentRouter;

impl IntentRouter {
    /// Route a user request to the most appropriate agent.
    ///
    /// Builds a system prompt from the available agent descriptors, sends the
    /// user's input (with recent history for context) to the LLM, and parses
    /// the response to extract an agent id.
    pub async fn route(
        agents: &[&DynAgent],
        user_input: &str,
        chat_history: &[Message],
        provider: &dyn LlmProvider,
        provider_config: &ProviderConfig,
        cancel_token: &CancellationToken,
        current_agent_id: Option<&str>,
    ) -> Result<RoutingDecision, RouterError> {
        if cancel_token.is_cancelled() {
            return Err(RouterError::Cancelled);
        }

        let system_prompt = Self::build_system_prompt(agents, current_agent_id);
        let agent_ids: Vec<&str> = agents.iter().map(|a| a.descriptor().id).collect();

        let mut messages = vec![Message::text(Role::System, &system_prompt)];

        // Include last 4 history messages for context.
        let history_tail = if chat_history.len() > 4 {
            &chat_history[chat_history.len() - 4..]
        } else {
            chat_history
        };
        messages.extend_from_slice(history_tail);
        messages.push(Message::text(Role::User, user_input));

        let request = ChatRequest {
            model: provider_config.model.clone(),
            messages,
            max_tokens: Some(50),
            temperature: Some(0.0),
            stream: Some(false),
            ..Default::default()
        };

        let response = tokio::select! {
            _ = cancel_token.cancelled() => {
                return Err(RouterError::Cancelled);
            }
            result = provider.chat(&request) => {
                result.map_err(|e| RouterError::LlmError(e.to_string()))?
            }
        };

        Self::parse_response(&response, &agent_ids)
    }

    fn build_system_prompt(agents: &[&DynAgent], current_agent_id: Option<&str>) -> String {
        let mut prompt = String::from(
            "You are an intent router. Given the user's message, decide which agent should handle it.\n\
             Available agents:\n\n",
        );

        for agent in agents {
            let desc = agent.descriptor();
            prompt.push_str(&format!("- id: \"{}\"\n", desc.id));
            prompt.push_str(&format!("  name: {}\n", desc.display_name));
            prompt.push_str(&format!("  description: {}\n", desc.description));
            if !desc.examples.is_empty() {
                prompt.push_str("  examples: ");
                prompt.push_str(&desc.examples.join(", "));
                prompt.push('\n');
            }
            prompt.push('\n');
        }

        // 当存在绑定 agent 时，增加会话连续性指导
        if let Some(agent_id) = current_agent_id {
            prompt.push_str(&format!(
                "IMPORTANT: The current conversation is being handled by agent \"{agent_id}\". \
                 If the user's message is a follow-up, continuation, or refinement of the previous topic, \
                 you should keep routing to \"{agent_id}\". \
                 Only route to a different agent if the user clearly switches to a new, unrelated topic.\n\n",
            ));
        }

        prompt.push_str(
            "Respond with ONLY a JSON object: {\"agent_id\": \"<id>\"}\n\
             Do not include any other text.",
        );

        prompt
    }

    fn parse_response(response: &str, valid_ids: &[&str]) -> Result<RoutingDecision, RouterError> {
        let trimmed = response.trim();

        let try_extract = |val: &serde_json::Value| -> Option<RoutingDecision> {
            let id = val.get("agent_id")?.as_str()?;
            valid_ids.contains(&id).then(|| RoutingDecision {
                agent_id: id.to_string(),
            })
        };

        // Strategy 1: Direct JSON parse.
        if let Some(decision) = serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .and_then(|val| try_extract(&val))
        {
            return Ok(decision);
        }

        // Strategy 2: Extract a JSON fragment from the response.
        if let Some(start) = trimmed.find('{')
            && let Some(end) = trimmed[start..].find('}')
        {
            let fragment = &trimmed[start..=start + end];
            if let Some(decision) = serde_json::from_str::<serde_json::Value>(fragment)
                .ok()
                .and_then(|val| try_extract(&val))
            {
                return Ok(decision);
            }
        }

        // Strategy 3: Check if the response text contains any valid agent id.
        let lower = trimmed.to_lowercase();
        for &id in valid_ids {
            if lower.contains(id) {
                return Ok(RoutingDecision {
                    agent_id: id.to_string(),
                });
            }
        }

        Err(RouterError::ParseFailed(format!(
            "Could not extract agent_id from response: {}",
            trimmed
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_direct_json() {
        let ids = vec!["general_chat", "sql_agent"];
        let result = IntentRouter::parse_response(r#"{"agent_id": "sql_agent"}"#, &ids);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().agent_id, "sql_agent");
    }

    #[test]
    fn test_parse_response_embedded_json() {
        let ids = vec!["general_chat", "sql_agent"];
        let result =
            IntentRouter::parse_response(r#"The best agent is {"agent_id": "general_chat"}"#, &ids);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().agent_id, "general_chat");
    }

    #[test]
    fn test_parse_response_contains_id() {
        let ids = vec!["general_chat", "sql_agent"];
        let result = IntentRouter::parse_response("I think general_chat is best", &ids);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().agent_id, "general_chat");
    }

    #[test]
    fn test_parse_response_invalid() {
        let ids = vec!["general_chat", "sql_agent"];
        let result = IntentRouter::parse_response("no idea", &ids);
        assert!(result.is_err());
    }
}
