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

    /// 会话亲和性是否仍有效（未超过轮次上限）。
    pub fn is_valid(&self) -> bool {
        self.current_agent_id.is_some() && self.consecutive_rounds < MAX_AFFINITY_ROUNDS
    }
}

/// Agent dispatcher (LLM-first routing):
///
/// 1. **Single-candidate shortcut** — if only 0 or 1 agent is available, skip routing
/// 2. **LLM intent routing** — ask the LLM to pick the best agent
/// 3. **Rule-based fallback** — used only when LLM routing is unavailable/failed
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

        // --- Level 1: Single candidate shortcut ---
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

        // --- Level 1.5: 亲和性快速路径 ---
        // 亲和性有效 + 输入看起来像追问 + 当前 agent 仍在可用列表中 → 跳过 LLM 路由
        if affinity.is_valid() {
            if let Some(current_id) = affinity.current_agent_id().map(str::to_owned) {
                if Self::looks_like_followup(&ctx.user_input) {
                    if let Some(agent) =
                        available.iter().find(|a| a.descriptor().id == current_id)
                    {
                        info!(
                            agent = current_id.as_str(),
                            "Dispatched via session affinity fast-path (follow-up detected)"
                        );
                        affinity.bind(&current_id);
                        let agent = Arc::clone(agent);
                        tokio::spawn(async move {
                            agent.execute(ctx, tx).await;
                        });
                        return rx;
                    }
                }
            }
        }

        // --- Level 2: LLM intent routing ---
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
                    &ctx.provider_config,
                    &ctx.cancel_token,
                    affinity.current_agent_id(),
                )
                .await
                {
                    Ok(decision) => {
                        if let Some(agent) = registry.get(&decision.agent_id) {
                            info!(
                                agent = decision.agent_id.as_str(),
                                "Dispatched via LLM routing"
                            );
                            Arc::clone(agent)
                        } else {
                            warn!(
                                agent = decision.agent_id.as_str(),
                                "LLM routed to unknown agent, falling back"
                            );
                            Arc::clone(available[0])
                        }
                    }
                    Err(e) => Self::fallback_agent(&ctx.user_input, &available, affinity, &e),
                }
            }
            Err(e) => Self::fallback_agent(&ctx.user_input, &available, affinity, &e),
        };

        let agent_id = agent.descriptor().id;
        affinity.bind(agent_id);
        tokio::spawn(async move {
            agent.execute(ctx, tx).await;
        });

        rx
    }

    fn fallback_agent<'a>(
        user_input: &str,
        available: &[&'a DynAgent],
        affinity: &SessionAffinity,
        error: &impl std::fmt::Display,
    ) -> DynAgent {
        warn!(error = %error, "LLM routing unavailable, falling back to rule-based routing");
        if let Some(agent) = Self::rule_match(user_input, available, affinity) {
            let agent_id = agent.descriptor().id;
            info!(agent = agent_id, "Dispatched via rule fallback");
            Arc::clone(agent)
        } else {
            Arc::clone(available[0])
        }
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
        if affinity.is_valid()
            && let Some(id) = affinity.current_agent_id()
        {
            for agent in available {
                if agent.descriptor().id == id {
                    return Some(agent);
                }
            }
        }

        None
    }

    /// 判断用户输入是否看起来像追问/延续消息。
    ///
    /// 包含命令前缀时返回 false（用户明确发起新意图），
    /// 短消息或包含追问关键词时返回 true，其他情况返回 false（走 LLM 路由）。
    fn looks_like_followup(user_input: &str) -> bool {
        let trimmed = user_input.trim();

        // 包含命令前缀 → 不是追问，是明确的新意图
        if trimmed.starts_with('/') {
            return false;
        }

        // 短消息（≤30 字符）大概率是追问
        if trimmed.chars().count() <= 30 {
            return true;
        }

        // 包含追问关键词
        let followup_keywords = [
            "继续",
            "还有",
            "改一下",
            "换成",
            "再来",
            "接着",
            "然后呢",
            "补充",
            "修改",
            "调整",
            "再试",
            "重新",
            "另外",
            "对了",
            "顺便",
            "还要",
            "加上",
            "去掉",
            "不要",
            "改为",
            "continue",
            "refine",
            "also",
            "more",
            "again",
            "update",
            "change",
            "modify",
            "keep going",
            "go on",
        ];
        let input_lower = trimmed.to_lowercase();
        for kw in &followup_keywords {
            if input_lower.contains(kw) {
                return true;
            }
        }

        false
    }
}
