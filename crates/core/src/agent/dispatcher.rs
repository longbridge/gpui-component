use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, warn};

use super::registry::AgentRegistry;
use super::router::IntentRouter;
use super::types::{AgentContext, AgentEvent, DynAgent};

/// Maximum consecutive rounds an agent can stay bound via session affinity.
const MAX_AFFINITY_ROUNDS: u32 = 10;

/// Tracks which agent is currently "bound" to a conversation session
/// so that follow-up messages stay with the same agent.
#[derive(Debug, Clone, Default)]
pub struct SessionAffinity {
    current_agent_id: Option<String>,
    consecutive_rounds: u32,
}

impl SessionAffinity {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind to a specific agent and reset the round counter.
    pub fn bind(&mut self, agent_id: &str) {
        if self.current_agent_id.as_deref() == Some(agent_id) {
            self.consecutive_rounds += 1;
        } else {
            self.current_agent_id = Some(agent_id.to_string());
            self.consecutive_rounds = 1;
        }
    }

    /// Clear affinity, forcing a fresh routing decision.
    pub fn reset(&mut self) {
        self.current_agent_id = None;
        self.consecutive_rounds = 0;
    }

    /// The currently bound agent id, if any.
    pub fn current_agent_id(&self) -> Option<&str> {
        self.current_agent_id.as_deref()
    }

    /// Whether the affinity is still valid (has not exceeded the round limit).
    fn is_valid(&self) -> bool {
        self.current_agent_id.is_some() && self.consecutive_rounds < MAX_AFFINITY_ROUNDS
    }
}

/// Three-level agent dispatcher:
///
/// 1. **Rule match** — command prefix, keywords, session affinity
/// 2. **Single-candidate shortcut** — if only 0 or 1 agent is available, skip LLM routing
/// 3. **LLM intent routing** — ask the LLM to pick the best agent
pub struct AgentDispatcher;

impl AgentDispatcher {
    /// Dispatch a user request to the appropriate agent.
    ///
    /// Returns an `mpsc::Receiver<AgentEvent>` that the caller can poll for
    /// streamed results (text deltas, progress, completion, errors).
    pub async fn dispatch(
        ctx: AgentContext,
        registry: &AgentRegistry,
        affinity: &mut SessionAffinity,
    ) -> mpsc::Receiver<AgentEvent> {
        let (tx, rx) = mpsc::channel(64);

        let available = registry.available_agents(&ctx);

        if available.is_empty() {
            let _ = tx
                .send(AgentEvent::Error("No agents available".to_string()))
                .await;
            return rx;
        }

        // --- Level 1: Rule-based matching ---
        if let Some(agent) = Self::rule_match(&ctx.user_input, &available, affinity) {
            let agent_id = agent.descriptor().id;
            info!(agent = agent_id, "Dispatched via rule match");
            affinity.bind(agent_id);
            let agent = Arc::clone(agent);
            tokio::spawn(async move {
                agent.execute(ctx, tx).await;
            });
            return rx;
        }

        // --- Level 2: Single candidate shortcut ---
        if available.len() == 1 {
            let agent = available[0];
            let agent_id = agent.descriptor().id;
            info!(agent = agent_id, "Dispatched as sole available agent");
            affinity.bind(agent_id);
            let agent = Arc::clone(agent);
            tokio::spawn(async move {
                agent.execute(ctx, tx).await;
            });
            return rx;
        }

        // --- Level 3: LLM intent routing ---
        let provider_result = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await;

        let agent = match provider_result {
            Ok(provider) => {
                match IntentRouter::route(
                    &available,
                    &ctx.user_input,
                    &ctx.chat_history,
                    provider.as_ref(),
                    &ctx.cancel_token,
                )
                .await
                {
                    Ok(decision) => {
                        if let Some(agent) = registry.get(&decision.agent_id) {
                            info!(agent = decision.agent_id.as_str(), "Dispatched via LLM routing");
                            Arc::clone(agent)
                        } else {
                            warn!(
                                agent = decision.agent_id.as_str(),
                                "LLM routed to unknown agent, falling back"
                            );
                            Arc::clone(available[0])
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "LLM routing failed, falling back to first available");
                        Arc::clone(available[0])
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to get provider for routing, falling back");
                Arc::clone(available[0])
            }
        };

        let agent_id = agent.descriptor().id;
        affinity.bind(agent_id);
        tokio::spawn(async move {
            agent.execute(ctx, tx).await;
        });

        rx
    }

    /// Level 1: Rule-based matching.
    ///
    /// Checks command prefix first, then keywords, then session affinity.
    fn rule_match<'a>(
        user_input: &str,
        available: &[&'a DynAgent],
        affinity: &SessionAffinity,
    ) -> Option<&'a DynAgent> {
        let input_lower = user_input.to_lowercase();

        // 1a: Command prefix match (e.g. "/sql SELECT ...")
        for agent in available {
            if agent
                .descriptor()
                .command_prefix
                .is_some_and(|prefix| input_lower.starts_with(prefix))
            {
                return Some(agent);
            }
        }

        // 1b: Keyword match — pick the agent with the most keyword hits.
        let mut best: Option<(&DynAgent, usize)> = None;
        for agent in available {
            let hits = agent
                .descriptor()
                .keywords
                .iter()
                .filter(|kw| input_lower.contains(&kw.to_lowercase()))
                .count();
            if hits > 0 && best.is_none_or(|(_, prev_hits)| hits > prev_hits) {
                best = Some((agent, hits));
            }
        }
        if let Some((agent, _)) = best {
            return Some(agent);
        }

        // 1c: Session affinity (if still valid).
        if affinity.is_valid() && let Some(id) = affinity.current_agent_id() {
            for agent in available {
                if agent.descriptor().id == id {
                    return Some(agent);
                }
            }
        }

        None
    }
}
