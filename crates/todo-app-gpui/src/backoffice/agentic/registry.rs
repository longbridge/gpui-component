use super::*;
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct Registry<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> {
    agents: HashMap<String, AiAgent<M, L>>,
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Registry<M, L> {
    /// 注册 Agent
    pub fn register_agent(&mut self, agent: AiAgent<M, L>) {
        self.agents
            .insert(agent.execution_context().session_id.clone(), agent);
    }

    /// 获取 Agent
    pub fn get_agent(&self, name: &str) -> Option<&AiAgent<M, L>> {
        self.agents.get(name)
    }

    /// 获取所有 Agent
    pub fn all_agents(&self) -> Vec<&AiAgent<M, L>> {
        self.agents.values().collect()
    }

    /// 获取可变引用的 Agent
    pub fn get_agent_mut(&mut self, name: &str) -> Option<&mut AiAgent<M, L>> {
        self.agents.get_mut(name)
    }

    /// 移除 Agent
    pub fn remove_agent(&mut self, name: &str) -> Option<AiAgent<M, L>> {
        self.agents.remove(name)
    }

    /// 列出所有 Agent 的会话 ID
    pub fn list_session_ids(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    /// 获取 Agent 数量
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// 检查 Agent 是否存在
    pub fn contains_agent(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    /// 清空所有 Agent
    pub fn clear_agents(&mut self) {
        self.agents.clear();
    }

    /// 批量注册 Agent
    pub fn register_agents(&mut self, agents: Vec<AiAgent<M, L>>) {
        for agent in agents {
            self.register_agent(agent);
        }
    }

    /// 根据状态过滤 Agent
    pub fn filter_agents_by_status(&self, status: AgentStatus) -> Vec<&AiAgent<M, L>> {
        self.agents
            .values()
            .filter(|agent| agent.execution_context().status == status)
            .collect()
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> AgentStatistics {
        let mut stats = AgentStatistics::default();
        stats.total_agents = self.agents.len();

        for agent in self.agents.values() {
            let context = agent.execution_context();
            match context.status {
                AgentStatus::Running => stats.running_agents += 1,
                AgentStatus::Idle => stats.idle_agents += 1,
                AgentStatus::Paused => stats.paused_agents += 1,
                AgentStatus::Stopped => stats.stopped_agents += 1,
                AgentStatus::Error => stats.error_agents += 1,
            }
        }

        stats
    }

    /// 查找匹配条件的 Agent
    pub fn find_agents(&self, predicate: impl Fn(&AiAgent<M, L>) -> bool) -> Vec<&AiAgent<M, L>> {
        self.agents
            .values()
            .filter(|agent| predicate(agent))
            .collect()
    }

    /// 更新 Agent 状态
    pub fn update_agent_status(&mut self, session_id: &str, status: AgentStatus) -> bool {
        if let Some(agent) = self.agents.get_mut(session_id) {
            agent.update_status(status);
            true
        } else {
            false
        }
    }

    /// 清理非活跃的 Agent
    pub fn cleanup_inactive_agents(&mut self, max_idle_duration: std::time::Duration) -> usize {
        let now = std::time::SystemTime::now();
        let mut to_remove = Vec::new();

        for (session_id, agent) in &self.agents {
            let context = agent.execution_context();

            if matches!(context.status, AgentStatus::Idle | AgentStatus::Stopped) {
                if let Ok(duration) = now.duration_since(context.last_activity) {
                    if duration > max_idle_duration {
                        to_remove.push(session_id.clone());
                    }
                }
            }
        }

        let removed_count = to_remove.len();
        for session_id in to_remove {
            self.agents.remove(&session_id);
        }

        removed_count
    }
}

/// Agent 统计信息
#[derive(Debug, Clone, Default)]
pub struct AgentStatistics {
    pub total_agents: usize,
    pub running_agents: usize,
    pub idle_agents: usize,
    pub paused_agents: usize,
    pub stopped_agents: usize,
    pub error_agents: usize,
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Actor for Registry<M, L> {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("Agent Registry started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("Agent Registry stopped");
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Supervised
    for Registry<M, L>
{
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        tracing::warn!("Agent Registry restarting");
    }
}

impl<M: Memory + Send + Unpin + Default + 'static, L: LLM + Send + Unpin + Default + 'static>
    SystemService for Registry<M, L>
{
}

// Actix 消息定义
#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterAgent<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> {
    pub agent: AiAgent<M, L>,
}

#[derive(Message)]
#[rtype(result = "bool")]
pub struct RemoveAgent {
    pub session_id: String,
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct ListAgents;

#[derive(Message)]
#[rtype(result = "AgentStatistics")]
pub struct GetStatistics;

#[derive(Message)]
#[rtype(result = "bool")]
pub struct ContainsAgent {
    pub session_id: String,
}

#[derive(Message)]
#[rtype(result = "usize")]
pub struct GetAgentCount;

#[derive(Message)]
#[rtype(result = "usize")]
pub struct CleanupInactiveAgents {
    pub max_idle_duration: std::time::Duration,
}

#[derive(Message)]
#[rtype(result = "bool")]
pub struct UpdateAgentStatus {
    pub session_id: String,
    pub status: AgentStatus,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClearAllAgents;

// Actix 消息处理器
impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static>
    Handler<RegisterAgent<M, L>> for Registry<M, L>
{
    type Result = ();

    fn handle(&mut self, msg: RegisterAgent<M, L>, _ctx: &mut Self::Context) -> Self::Result {
        let session_id = msg.agent.execution_context().session_id.clone();
        self.register_agent(msg.agent);
        tracing::info!("Registered agent with session_id: {}", session_id);
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<RemoveAgent>
    for Registry<M, L>
{
    type Result = bool;

    fn handle(&mut self, msg: RemoveAgent, _ctx: &mut Self::Context) -> Self::Result {
        if self.remove_agent(&msg.session_id).is_some() {
            tracing::info!("Removed agent with session_id: {}", msg.session_id);
            true
        } else {
            tracing::warn!("Agent with session_id '{}' not found", msg.session_id);
            false
        }
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<ListAgents>
    for Registry<M, L>
{
    type Result = Vec<String>;

    fn handle(&mut self, _msg: ListAgents, _ctx: &mut Self::Context) -> Self::Result {
        self.list_session_ids()
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<GetStatistics>
    for Registry<M, L>
{
    type Result = AgentStatistics;

    fn handle(&mut self, _msg: GetStatistics, _ctx: &mut Self::Context) -> Self::Result {
        self.get_statistics()
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<ContainsAgent>
    for Registry<M, L>
{
    type Result = bool;

    fn handle(&mut self, msg: ContainsAgent, _ctx: &mut Self::Context) -> Self::Result {
        self.contains_agent(&msg.session_id)
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<GetAgentCount>
    for Registry<M, L>
{
    type Result = usize;

    fn handle(&mut self, _msg: GetAgentCount, _ctx: &mut Self::Context) -> Self::Result {
        self.agent_count()
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static>
    Handler<CleanupInactiveAgents> for Registry<M, L>
{
    type Result = usize;

    fn handle(&mut self, msg: CleanupInactiveAgents, _ctx: &mut Self::Context) -> Self::Result {
        let removed = self.cleanup_inactive_agents(msg.max_idle_duration);
        tracing::info!("Cleaned up {} inactive agents", removed);
        removed
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<UpdateAgentStatus>
    for Registry<M, L>
{
    type Result = bool;

    fn handle(&mut self, msg: UpdateAgentStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.update_agent_status(&msg.session_id, msg.status)
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Handler<ClearAllAgents>
    for Registry<M, L>
{
    type Result = ();

    fn handle(&mut self, _msg: ClearAllAgents, _ctx: &mut Self::Context) -> Self::Result {
        let count = self.agent_count();
        self.clear_agents();
        tracing::info!("Cleared all {} agents", count);
    }
}
