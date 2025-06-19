use crate::models::{
    mcp_config::{McpProviderInfo, McpProviderManager},
    provider_config::{LlmProviderInfo, LlmProviderManager, ModelInfo},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Todo状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub enum TodoStatus {
    Todo,       // 待办
    InProgress, // 进行中
    Done,       // 已完成
    Alert,      // 警报
    Cancelled,  // 已取消
}

impl TodoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Todo => "待办",
            TodoStatus::InProgress => "进行中",
            TodoStatus::Alert => "警报",
            TodoStatus::Done => "已完成",
            TodoStatus::Cancelled => "已取消",
        }
    }

    pub fn all() -> Vec<&'static str> {
        vec!["待办", "进行中", "警报", "已完成", "已取消"]
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "待办" => TodoStatus::Todo,
            "进行中" => TodoStatus::InProgress,
            "警报" => TodoStatus::Alert,
            "已完成" => TodoStatus::Done,
            "已取消" => TodoStatus::Cancelled,
            _ => TodoStatus::Todo,
        }
    }
}

/// 上传文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoFile {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub mime_type: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub uploaded_at: DateTime<Utc>,
}

/// 选中的模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedModel {
    pub provider_id: String,
    pub model_id: String,
    pub model_name: String,
    pub provider_name: String,
}

/// 选中的MCP工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedTool {
    pub provider_id: String,
    pub tool_name: String,
    pub provider_name: String,
    pub description: String,
}

/// Todo项目主结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TodoStatus,
    // AI配置
    pub selected_models: Vec<SelectedModel>,
    pub selected_tools: Vec<SelectedTool>,
    // 文件附件
    pub files: Vec<TodoFile>,
    // 配置选项
    pub recurring_enabled: bool,
    pub recurring_pattern: Option<String>, // "daily", "weekly", "monthly", "yearly"
    pub auto_execute: bool,
    pub enable_notifications: bool,
    pub push_to_feishu: bool,
    // 执行结果
    pub execution_logs: Vec<String>,
    pub last_execution_result: Option<String>,
    // 时间戳
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub reminder_date: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl Default for Todo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: String::new(),
            description: String::new(),
            status: TodoStatus::Todo,
            created_at: now,
            updated_at: now,
            due_date: None,
            reminder_date: None,
            completed_at: None,
            selected_models: Vec::new(),
            selected_tools: Vec::new(),
            files: Vec::new(),
            recurring_enabled: false,
            recurring_pattern: None,
            auto_execute: false,
            enable_notifications: true,
            push_to_feishu: false,
            execution_logs: Vec::new(),
            last_execution_result: None,
        }
    }
}

impl Todo {
    /// 创建新的Todo项目
    pub fn new(title: String, description: String) -> Self {
        Self {
            title,
            description,
            ..Default::default()
        }
    }

    /// 标记为完成
    pub fn mark_completed(&mut self) {
        self.status = TodoStatus::Done;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 标记为未完成
    pub fn mark_incomplete(&mut self) {
        if self.status == TodoStatus::Done {
            self.status = TodoStatus::Todo;
            self.completed_at = None;
            self.updated_at = Utc::now();
        }
    }

    /// 切换完成状态
    pub fn toggle_completed(&mut self) {
        if self.status == TodoStatus::Done {
            self.mark_incomplete();
        } else {
            self.mark_completed();
        }
    }

    /// 更新状态
    pub fn set_status(&mut self, status: TodoStatus) {
        let old_status = self.status;
        self.status = status;
        if status == TodoStatus::Done && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        } else if status != TodoStatus::Done && old_status == TodoStatus::Done {
            self.completed_at = None;
        }
        self.updated_at = Utc::now();
    }

    /// 添加选中的模型
    pub fn add_selected_model(
        &mut self,
        provider_manager: &LlmProviderManager,
        provider_id: &str,
        model_id: &str,
    ) -> anyhow::Result<()> {
        if let Some(provider) = provider_manager.get_provider(provider_id) {
            if let Some(model) = provider.models.iter().find(|m| m.id == model_id) {
                let selected_model = SelectedModel {
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                    model_name: model.display_name.clone(),
                    provider_name: provider.name.clone(),
                };

                // 检查是否已存在
                if !self
                    .selected_models
                    .iter()
                    .any(|m| m.provider_id == provider_id && m.model_id == model_id)
                {
                    self.selected_models.push(selected_model);
                    self.updated_at = Utc::now();
                }
                return Ok(());
            }
        }
        Err(anyhow::anyhow!(
            "Model not found: {}/{}",
            provider_id,
            model_id
        ))
    }

    /// 移除选中的模型
    pub fn remove_selected_model(&mut self, provider_id: &str, model_id: &str) {
        self.selected_models
            .retain(|m| !(m.provider_id == provider_id && m.model_id == model_id));
        self.updated_at = Utc::now();
    }

    /// 清空所有选中的模型
    pub fn clear_selected_models(&mut self) {
        self.selected_models.clear();
        self.updated_at = Utc::now();
    }

    /// 添加选中的工具
    pub fn add_selected_tool(
        &mut self,
        mcp_manager: &McpProviderManager,
        provider_id: &str,
        tool_name: &str,
    ) -> anyhow::Result<()> {
        if let Some(provider) = mcp_manager.get_provider(provider_id) {
            if let Some(tool) = provider.tools.iter().find(|t| t.name == tool_name) {
                let selected_tool = SelectedTool {
                    provider_id: provider_id.to_string(),
                    tool_name: tool_name.to_string(),
                    provider_name: provider.name.clone(),
                    description: tool.description.clone(),
                };

                // 检查是否已存在
                if !self
                    .selected_tools
                    .iter()
                    .any(|t| t.provider_id == provider_id && t.tool_name == tool_name)
                {
                    self.selected_tools.push(selected_tool);
                    self.updated_at = Utc::now();
                }
                return Ok(());
            }
        }
        Err(anyhow::anyhow!(
            "Tool not found: {}/{}",
            provider_id,
            tool_name
        ))
    }

    /// 移除选中的工具
    pub fn remove_selected_tool(&mut self, provider_id: &str, tool_name: &str) {
        self.selected_tools
            .retain(|t| !(t.provider_id == provider_id && t.tool_name == tool_name));
        self.updated_at = Utc::now();
    }

    /// 清空所有选中的工具
    pub fn clear_selected_tools(&mut self) {
        self.selected_tools.clear();
        self.updated_at = Utc::now();
    }

    /// 添加文件
    pub fn add_file(
        &mut self,
        name: String,
        path: String,
        size: Option<u64>,
        mime_type: Option<String>,
    ) {
        let file = TodoFile {
            name,
            path,
            size,
            mime_type,
            uploaded_at: Utc::now(),
        };
        self.files.push(file);
        self.updated_at = Utc::now();
    }

    /// 移除文件
    pub fn remove_file(&mut self, path: &str) {
        self.files.retain(|f| f.path != path);
        self.updated_at = Utc::now();
    }

    /// 添加执行日志
    pub fn add_execution_log(&mut self, log: String) {
        self.execution_logs.push(format!(
            "[{}] {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            log
        ));
        self.updated_at = Utc::now();
    }

    /// 设置执行结果
    pub fn set_execution_result(&mut self, result: String) {
        self.last_execution_result = Some(result);
        self.updated_at = Utc::now();
    }

    /// 检查是否过期
    pub fn is_overdue(&self) -> bool {
        if let Some(due_date) = self.due_date {
            due_date < Utc::now() && self.status != TodoStatus::Done
        } else {
            false
        }
    }

    /// 检查是否需要提醒
    pub fn needs_reminder(&self) -> bool {
        if let Some(reminder_date) = self.reminder_date {
            reminder_date <= Utc::now() && self.status != TodoStatus::Done
        } else {
            false
        }
    }

    /// 获取模型能力总结
    pub fn get_model_capabilities_summary(
        &self,
        provider_manager: &LlmProviderManager,
    ) -> Vec<String> {
        let mut capabilities = Vec::new();
        for selected_model in &self.selected_models {
            if let Some(provider) = provider_manager.get_provider(&selected_model.provider_id) {
                if let Some(model) = provider
                    .models
                    .iter()
                    .find(|m| m.id == selected_model.model_id)
                {
                    for capability in &model.capabilities {
                        let cap_str = capability.as_str().to_string();
                        if !capabilities.contains(&cap_str) {
                            capabilities.push(cap_str);
                        }
                    }
                }
            }
        }
        capabilities
    }

    /// 获取工具能力总结
    pub fn get_tool_capabilities_summary(&self) -> Vec<String> {
        self.selected_tools
            .iter()
            .map(|tool| tool.description.clone())
            .collect()
    }
}

/// Todo管理器
const TODO_CONFIG_FILE: &str = "config/todos.yml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoManager {
    #[serde(default)]
    pub todos: Vec<Todo>,
}

impl TodoManager {
    /// 从文件加载配置
    pub fn load() -> Self {
        let config_path = Path::new(TODO_CONFIG_FILE);
        if !config_path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_yaml::from_str::<Vec<Todo>>(&content) {
                Ok(todos) => Self { todos },
                Err(e) => {
                    eprintln!("Failed to parse Todo config: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("Failed to read Todo config file: {}", e);
                Self::default()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Path::new(TODO_CONFIG_FILE);

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(&self.todos)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有Todo列表
    pub fn list_todos(&self) -> Vec<Todo> {
        // 按更新时间倒序排列
        let mut todos = self.todos.clone();
        todos.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        todos
    }

    /// 根据ID查询Todo
    pub fn get_todo(&self, id: &str) -> Option<&Todo> {
        self.todos.iter().find(|todo| todo.id == id)
    }

    /// 根据状态筛选Todo
    pub fn get_todos_by_status(&self, status: TodoStatus) -> Vec<Todo> {
        self.todos
            .iter()
            .filter(|todo| todo.status == status)
            .cloned()
            .collect()
    }

    /// 获取过期的Todo
    pub fn get_overdue_todos(&self) -> Vec<Todo> {
        self.todos
            .iter()
            .filter(|todo| todo.is_overdue())
            .cloned()
            .collect()
    }

    /// 获取需要提醒的Todo
    pub fn get_reminder_todos(&self) -> Vec<Todo> {
        self.todos
            .iter()
            .filter(|todo| todo.needs_reminder())
            .cloned()
            .collect()
    }

    /// 添加新的Todo
    pub fn add_todo(&mut self, todo: Todo) -> &mut Self {
        self.todos.push(todo);
        self
    }

    /// 更新Todo
    pub fn update_todo(&mut self, id: &str, mut todo: Todo) -> anyhow::Result<&mut Self> {
        if let Some(position) = self.todos.iter().position(|t| t.id == id) {
            todo.updated_at = Utc::now();
            self.todos[position] = todo;
        } else {
            return Err(anyhow::anyhow!("Todo with id '{}' not found", id));
        }
        Ok(self)
    }

    /// 删除Todo
    pub fn delete_todo(&mut self, id: &str) -> Option<Todo> {
        if let Some(position) = self.todos.iter().position(|t| t.id == id) {
            let todo = self.todos.remove(position);
            return Some(todo);
        }
        None
    }

    /// 批量删除Todo
    pub fn batch_delete(&mut self, ids: &[String]) -> Vec<Todo> {
        let mut deleted = Vec::new();
        for id in ids {
            if let Some(position) = self.todos.iter().position(|t| t.id == *id) {
                let todo = self.todos.remove(position);
                deleted.push(todo);
            }
        }
        deleted
    }

    /// 搜索Todo
    pub fn search_todos(&self, query: &str) -> Vec<Todo> {
        let query_lower = query.to_lowercase();
        self.todos
            .iter()
            .filter(|todo| {
                todo.title.to_lowercase().contains(&query_lower)
                    || todo.description.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// 获取Todo统计信息
    pub fn get_statistics(&self) -> TodoStatistics {
        let total = self.todos.len();
        let completed = self
            .todos
            .iter()
            .filter(|t| t.status == TodoStatus::Done)
            .count();
        let in_progress = self
            .todos
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        let todo = self
            .todos
            .iter()
            .filter(|t| t.status == TodoStatus::Todo)
            .count();
        let cancelled = self
            .todos
            .iter()
            .filter(|t| t.status == TodoStatus::Cancelled)
            .count();
        let overdue = self.todos.iter().filter(|t| t.is_overdue()).count();

        TodoStatistics {
            total,
            completed,
            in_progress,
            todo,
            cancelled,
            overdue,
        }
    }

    /// 清空所有Todo
    pub fn clear(&mut self) {
        self.todos.clear();
    }

    /// 获取Todo数量
    pub fn count(&self) -> usize {
        self.todos.len()
    }

    /// 创建大量伪造的测试数据 (1000条)
    pub fn create_fake_data() -> Self {
        let mut manager = TodoManager::default();

        // 生成1000条Todo数据
        let fake_todos: Vec<Todo> = (0..1000).map(|_| random_todo()).collect();

        // 将所有伪造的Todo添加到管理器中
        for todo in fake_todos {
            manager.todos.push(todo);
        }

        manager
    }
}

fn random_todo() -> Todo {
    use rand::prelude::*;

    // 100条常见待办事项模板
    static TITLES: &[&str] = &[
        "完成{}的日报",
        "联系{}",
        "整理{}资料",
        "参加{}会议",
        "审核{}文档",
        "准备{}演示",
        "更新{}进度",
        "提交{}申请",
        "学习{}课程",
        "预约{}",
        "购买办公用品",
        "检查邮件",
        "备份数据",
        "清理桌面",
        "安排下周计划",
        "阅读新通知",
        "回复客户信息",
        "完善项目文档",
        "测试新功能",
        "优化代码",
        "部署到生产环境",
        "修复Bug",
        "撰写周报",
        "整理会议纪要",
        "统计本月数据",
        "制定预算",
        "确认会议时间",
        "准备面试材料",
        "安排培训",
        "更新简历",
        "打扫卫生",
        "采购物资",
        "归还借用物品",
        "打印文件",
        "扫描合同",
        "签署协议",
        "上传资料",
        "下载报告",
        "同步进度",
        "提醒同事",
        "预约体检",
        "缴纳费用",
        "续签合同",
        "申请报销",
        "整理发票",
        "归档文件",
        "检查设备",
        "维护服务器",
        "更新软件",
        "更换密码",
        "设置权限",
        "备份数据库",
        "清理缓存",
        "巡检网络",
        "测试打印机",
        "检查安全隐患",
        "安排值班",
        "统计考勤",
        "整理客户名单",
        "发送通知",
        "准备礼品",
        "安排聚餐",
        "预定场地",
        "确认嘉宾",
        "制作宣传海报",
        "发布公告",
        "收集反馈",
        "分析数据",
        "制定KPI",
        "组织团建",
        "安排面谈",
        "准备PPT",
        "整理照片",
        "编辑视频",
        "上传作品",
        "申请加班",
        "审批请假",
        "安排轮休",
        "检查库存",
        "补充物料",
        "联系供应商",
        "核对账单",
        "催收款项",
        "安排发货",
        "确认收货",
        "处理投诉",
        "回访客户",
        "更新官网",
        "维护公众号",
        "发布推文",
        "整理代码",
        "合并分支",
        "代码评审",
        "编写测试用例",
        "运行自动化测试",
        "生成报告",
        "优化性能",
        "升级依赖",
        "修订计划",
        "总结经验",
        "制定目标",
        "安排复盘",
    ];

    static DESCS: &[&str] = &[
        "请在今天下班前完成该任务，并将结果通过邮件反馈给负责人。如有疑问请及时沟通，确保进度顺利推进。",
        "与{}详细沟通需求，整理会议纪要并上传至系统，确保团队成员都能及时了解最新进展。",
        "将与{}相关的所有资料进行分类整理，按时间顺序归档，并在资料库中建立索引以便后续查找。",
        "请于明天下午三点准时参加与{}的线上会议，提前准备需要讨论的议题和相关文档，会议结束后整理纪要。",
        "对{}提交的文档进行全面审核，重点检查数据准确性和逻辑完整性，发现问题及时反馈并协助修改。",
        "为即将到来的{}演示准备PPT和讲稿，确保内容详实、逻辑清晰，并提前进行彩排保证顺利进行。",
        "每日更新{}的项目进度，将最新进展同步到团队群，并在周会时进行简要汇报，确保信息同步。",
        "填写并提交{}相关的申请表格，确保所有信息准确无误，提交后请关注审批进度并及时跟进。",
        "利用业余时间学习{}课程，做好学习笔记，遇到不懂的问题及时向讲师或同学请教，提升专业能力。",
        "提前预约{}的相关服务，确认时间和地点，避免与其他重要事项冲突，如有变动请及时调整。",
        "根据实际需求填写办公用品采购清单，控制预算，采购完成后做好入库登记并通知相关同事领取。",
        "每天定时检查工作邮箱，及时回复重要邮件，对需要处理的事项做好标记，避免遗漏关键信息。",
        "定期备份重要数据文件，将备份文件存储在安全位置，并做好备份记录，防止数据丢失造成损失。",
        "每周五下班前清理办公桌面和电脑桌面，归还借用物品，保持办公环境整洁有序，提高工作效率。",
        "根据本周工作情况，制定下周详细计划，明确每项任务的负责人和截止时间，并在周一早会进行说明。",
        "认真阅读公司发布的新通知，了解最新政策和流程变化，确保自己的工作符合公司要求。",
        "及时回复客户的各类信息，耐心解答客户疑问，维护良好的客户关系，提升客户满意度。",
        "完善项目文档，补充缺失部分，确保文档结构清晰、内容详实，方便团队成员查阅和维护。",
        "对新开发的功能进行全面测试，记录测试结果和发现的问题，及时反馈给开发人员进行修复。",
        "对现有代码进行优化，提升运行效率和可维护性，优化后请进行回归测试确保功能正常。",
        "统计本月数据，整理成报表，分析关键指标，为下月计划和决策提供数据支持。",
        "制定部门预算，合理分配各项支出，确保资金使用高效合规，及时向财务部门报备。",
        "确认下周会议时间，提前通知参会人员，准备好相关资料，确保会议顺利进行。",
        "准备面试材料，整理候选人简历，安排面试时间并通知相关人员，确保流程顺畅。",
        "安排新员工培训，准备培训资料，确保培训内容覆盖岗位要求，帮助新员工快速上手。",
        "更新个人简历，补充近期项目经验，突出个人优势，提升求职竞争力。",
        "打扫办公室卫生，清理公共区域，营造良好工作环境，提升团队士气。",
        "采购日常物资，确保库存充足，满足团队日常需求，避免因物资短缺影响工作。",
        "归还借用物品，做好登记，避免物品遗失，保持物品管理有序。",
        "打印重要文件，检查内容无误后分发给相关人员，确保信息传递准确。",
        "扫描合同文件，保存电子版并归档，便于后续查找和管理，提升工作效率。",
        "签署合作协议，确认条款无误后完成签字流程，确保双方权益得到保障。",
        "上传项目资料至云盘，设置访问权限，确保数据安全并方便团队成员查阅。",
        "下载最新报告，阅读并整理要点，准备汇报材料，确保信息传递到位。",
        "同步项目进度，定期与团队成员沟通，解决遇到的问题，保证项目顺利推进。",
        "提醒同事完成分配任务，协助解决遇到的困难，促进团队协作。",
        "预约年度体检，确认时间地点，提前安排好工作，确保健康检查顺利进行。",
        "缴纳各项费用，保存缴费凭证，及时报销，确保财务流程合规。",
        "续签合同，确认条款变更，完成签署流程，确保合作关系持续稳定。",
        "申请差旅报销，整理发票和行程单，提交审批，确保报销流程顺畅。",
        "整理本月发票，分类归档，方便财务查账，提高工作效率。",
        "归档历史文件，建立索引，便于后续查找和管理，提升资料管理水平。",
        "检查办公设备运行状态，发现故障及时报修，保障日常工作顺利进行。",
        "维护服务器，定期检查系统安全和性能，防止出现故障影响业务。",
        "更新常用软件，确保版本最新，避免安全风险，提升工作效率。",
        "更换重要账号密码，提升账户安全性，防止信息泄露。",
        "设置系统权限，合理分配访问级别，保障数据安全和合规性。",
        "备份数据库，定期检查备份有效性，防止数据丢失造成损失。",
        "清理系统缓存，释放存储空间，提升运行速度和系统稳定性。",
        "巡检公司网络，排查安全隐患，确保网络畅通和数据安全。",
        "测试打印机功能，发现异常及时维修，保障日常办公需求。",
        "检查办公区域安全隐患，完善应急预案，提升安全管理水平。",
        "安排本月值班表，确保各时段有人值守，保障公司正常运转。",
        "统计员工考勤数据，核对异常情况，及时反馈并处理。",
        "整理客户名单，补充联系方式，便于后续跟进和客户维护。",
        "发送会议通知，附上议程和相关资料，确保参会人员提前知晓。",
        "准备节日礼品，提前采购并包装，安排发放，提升员工归属感。",
        "安排团队聚餐，预定餐厅，通知所有成员，增强团队凝聚力。",
        "预定会议场地，确认设备齐全，提前布置现场，确保会议顺利进行。",
        "确认活动嘉宾名单，发送邀请函，跟进回复，确保嘉宾准时出席。",
        "制作宣传海报，突出活动主题，设计美观大方，吸引更多参与者。",
        "发布公司公告，确保所有员工及时知晓，促进信息透明。",
        "收集活动反馈，整理成报告，提出改进建议，提升活动效果。",
        "分析销售数据，找出增长点，制定提升方案，推动业绩增长。",
        "制定下月KPI，明确考核标准，分解到个人，提升团队目标感。",
        "组织团队建设活动，增强成员凝聚力，提升团队协作能力。",
        "安排员工面谈，了解工作状态，收集建议，促进员工成长。",
        "准备项目PPT，内容简明扼要，突出重点，便于汇报展示。",
        "整理活动照片，分类存档，便于后续宣传和资料留存。",
        "编辑活动视频，剪辑精彩片段，制作成宣传片，提升活动影响力。",
        "上传作品至平台，完善描述和标签，提升曝光度和影响力。",
        "申请加班审批，说明原因和时长，等待领导批准，确保流程合规。",
        "审批员工请假申请，核对请假原因和时间，合理安排工作。",
        "安排轮休计划，确保工作正常运转，兼顾员工休息需求。",
        "检查仓库库存，补充短缺物料，避免断货影响生产。",
        "联系供应商补货，确认发货时间和数量，确保物资及时到位。",
        "核对本月账单，发现异常及时核实，确保账目清晰。",
        "催收未付款项，保持与客户沟通，确保资金及时回笼。",
        "安排发货事宜，确认收货地址和联系人，确保货物准时送达。",
        "确认客户收货情况，收集反馈，提升服务质量和客户满意度。",
        "处理客户投诉，耐心沟通，提出解决方案，维护公司形象。",
        "回访重点客户，了解需求，维护合作关系，促进业务发展。",
        "更新公司官网内容，发布最新动态，提升企业形象。",
        "维护公众号，定期推送优质内容，提升粉丝活跃度。",
        "发布产品推文，突出卖点，吸引潜在客户，促进销售转化。",
        "整理项目代码，优化结构，提升可读性和维护性。",
        "合并开发分支，解决冲突，确保主干代码稳定。",
        "进行代码评审，提出优化建议，提升代码质量。",
        "编写测试用例，覆盖主要功能，确保系统稳定可靠。",
        "运行自动化测试，记录结果，及时修复缺陷。",
        "生成测试报告，分析问题，制定改进措施，提升产品质量。",
        "优化系统性能，提升响应速度和并发能力，改善用户体验。",
        "升级依赖库，确保兼容性和安全性，减少潜在风险。",
        "修订项目计划，调整时间节点，合理分配资源，确保项目顺利推进。",
        "总结项目经验，整理成文档，便于团队学习和知识传承。",
        "制定下阶段目标，明确重点任务和负责人，提升执行力。",
        "安排项目复盘，总结得失，提出改进建议，持续优化流程。",
    ];

    // 人名列表
    static NAMES: &[&str] = &[
        "张伟", "王芳", "李娜", "刘强", "陈静", "杨洋", "赵敏", "孙丽", "周杰", "吴磊", "徐娟",
        "朱琳", "胡斌", "郭蕾", "何鹏", "高燕", "林峰", "罗晨", "梁薇", "宋涛", "唐雅", "韩冰",
        "冯军", "曹霞", "彭亮", "蒋丹", "谢飞", "邹兰", "石勇", "龙萍", "姚华", "康宁", "贺云",
        "薛凯", "雷鸣", "方玲", "洪武", "金秀", "常青", "毛莉", "王明", "李丽", "张强", "刘娟",
        "陈磊", "杨静", "赵刚", "孙鹏", "周萍", "吴敏", "徐涛", "朱华", "胡亮", "郭霞", "何军",
        "高丽", "林燕", "罗峰", "梁云", "宋雪", "唐勇", "韩玲", "冯娜", "曹斌", "彭艳", "蒋飞",
        "谢雯", "邹强", "石佳", "龙浩", "姚婷", "康杰", "贺芳", "薛宁", "雷琴", "方超", "洪梅",
        "金龙", "常欣", "毛健", "王蓉", "李帆", "张颖", "刘斌", "陈琪", "杨峰", "赵雪", "孙欢",
        "周雷", "吴娜", "徐杰", "朱敏", "胡玉", "郭文", "何倩", "高宇", "林鑫", "罗洁", "梁睿",
        "宋琦",
    ];

    // AI模型列表
    static AI_MODELS: &[(&str, &str, &str)] = &[
        ("openai", "gpt-4", "GPT-4"),
        ("openai", "gpt-3.5-turbo", "GPT-3.5 Turbo"),
        ("anthropic", "claude-3-sonnet", "Claude 3 Sonnet"),
        ("anthropic", "claude-3-haiku", "Claude 3 Haiku"),
        ("google", "gemini-pro", "Gemini Pro"),
        ("meta", "llama-2-70b", "Llama 2 70B"),
        ("mistral", "mistral-large", "Mistral Large"),
    ];

    // MCP工具列表
    static MCP_TOOLS: &[(&str, &str, &str)] = &[
        ("filesystem", "read_file", "读取文件内容"),
        ("filesystem", "write_file", "写入文件"),
        ("git", "diff", "查看代码差异"),
        ("git", "commit", "提交代码"),
        ("database", "query", "数据库查询"),
        ("calendar", "get_events", "获取日程安排"),
        ("email", "send", "发送邮件"),
        ("debugger", "memory_profiler", "内存性能分析"),
        ("linter", "check_code", "检查代码质量"),
        ("browser", "screenshot", "网页截图"),
    ];

    let mut rng = rand::rng();

    // 随机选择模板和人名
    let name = NAMES.choose(&mut rng).unwrap();
    let title_template = TITLES.choose(&mut rng).unwrap();
    let desc_template = DESCS.choose(&mut rng).unwrap();

    // 随机生成状态 (40%待办, 30%进行中, 25%已完成, 5%已取消)
    let status = match rng.random_range(0..100) {
        0..=39 => TodoStatus::Todo,
        40..=69 => TodoStatus::InProgress,
        70..=94 => TodoStatus::Done,
        _ => TodoStatus::Cancelled,
    };

    // 随机生成时间
    let days_ago = rng.random_range(0..=30);
    let hours_ago = rng.random_range(0..24);
    let created_at = Utc::now() - Duration::days(days_ago) - Duration::hours(hours_ago);
    let updated_at = created_at + Duration::hours(rng.random_range(1..=24));

    // 随机生成截止日期 (70%有截止日期)
    let due_date = if rng.random_bool(0.7) {
        let days_future = rng.random_range(-5..=30); // 可能过期
        Some(Utc::now() + Duration::days(days_future))
    } else {
        None
    };

    // 随机生成提醒日期
    let reminder_date = due_date.and_then(|due| {
        if rng.random_bool(0.6) {
            Some(due - Duration::days(rng.random_range(1..=3)))
        } else {
            None
        }
    });

    // 设置完成时间
    let completed_at = if status == TodoStatus::Done {
        Some(updated_at + Duration::hours(rng.random_range(1..=48)))
    } else {
        None
    };

    // 随机生成AI模型 (30%概率有模型)
    let selected_models = if rng.random_bool(0.3) {
        let model_count = rng.random_range(1..=3);
        (0..model_count)
            .map(|_| {
                let (provider_id, model_id, model_name) = AI_MODELS.choose(&mut rng).unwrap();
                SelectedModel {
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                    model_name: model_name.to_string(),
                    provider_name: match *provider_id {
                        "openai" => "OpenAI",
                        "anthropic" => "Anthropic",
                        "google" => "Google",
                        "meta" => "Meta",
                        "mistral" => "Mistral",
                        _ => "Unknown",
                    }
                    .to_string(),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // 随机生成MCP工具 (25%概率有工具)
    let selected_tools = if rng.random_bool(0.25) {
        let tool_count = rng.random_range(1..=4);
        (0..tool_count)
            .map(|_| {
                let (provider_id, tool_name, description) = MCP_TOOLS.choose(&mut rng).unwrap();
                SelectedTool {
                    provider_id: provider_id.to_string(),
                    tool_name: tool_name.to_string(),
                    provider_name: match *provider_id {
                        "filesystem" => "文件系统",
                        "git" => "Git",
                        "database" => "数据库",
                        "calendar" => "日历",
                        "email" => "邮件",
                        "debugger" => "调试器",
                        "linter" => "代码检查",
                        "browser" => "浏览器",
                        _ => "未知工具",
                    }
                    .to_string(),
                    description: description.to_string(),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // 随机生成文件 (15%概率有文件)
    let files = if rng.random_bool(0.15) {
        let file_count = rng.random_range(1..=3);
        let file_types = &[
            (
                "文档.docx",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                50000,
            ),
            ("报告.pdf", "application/pdf", 1200000),
            (
                "数据.xlsx",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                80000,
            ),
            ("代码.rs", "text/x-rust", 5000),
            ("截图.png", "image/png", 300000),
            (
                "演示.pptx",
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                2500000,
            ),
        ];

        (0..file_count)
            .map(|_| {
                let (name, mime_type, base_size) = file_types.choose(&mut rng).unwrap();
                let size_variation = rng.random_range(0.5..2.0);
                TodoFile {
                    name: name.to_string(),
                    path: format!("/files/{}", name),
                    size: Some(((*base_size as f64) * size_variation) as u64),
                    mime_type: Some(mime_type.to_string()),
                    uploaded_at: created_at + Duration::hours(rng.random_range(1..=12)),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // 随机生成执行日志 (40%概率有日志)
    let execution_logs = if rng.random_bool(0.4) {
        let log_count = rng.random_range(1..=5);
        let log_templates = &[
            "开始处理任务",
            "任务执行中...",
            "遇到问题，正在解决",
            "问题已解决，继续执行",
            "任务进度50%",
            "任务进度80%",
            "任务即将完成",
            "任务执行完成",
            "正在进行质量检查",
            "质量检查通过",
        ];

        (0..log_count)
            .map(|i| {
                let log_time = created_at + Duration::hours(i as i64 * 2);
                let log_msg = log_templates.choose(&mut rng).unwrap();
                format!("[{}] {}", log_time.format("%Y-%m-%d %H:%M:%S"), log_msg)
            })
            .collect()
    } else {
        Vec::new()
    };

    // 随机生成执行结果
    let last_execution_result = if !execution_logs.is_empty() && rng.random_bool(0.7) {
        let results = &[
            "任务执行成功",
            "任务部分完成",
            "任务执行中断",
            "等待审批",
            "需要进一步处理",
            "已转交给相关人员",
        ];
        Some(results.choose(&mut rng).unwrap().to_string())
    } else {
        None
    };

    Todo {
        id: uuid::Uuid::new_v4().to_string(),
        title: title_template.replace("{}", name),
        description: desc_template.replace("{}", name),
        status,
        created_at,
        updated_at,
        due_date,
        reminder_date,
        completed_at,
        selected_models,
        selected_tools,
        files,
        recurring_enabled: rng.random_bool(0.1), // 10%概率为循环任务
        recurring_pattern: if rng.random_bool(0.1) {
            let patterns = &["daily", "weekly", "monthly"];
            Some(patterns.choose(&mut rng).unwrap().to_string())
        } else {
            None
        },
        auto_execute: rng.random_bool(0.05), // 5%概率自动执行
        enable_notifications: rng.random_bool(0.8), // 80%概率启用通知
        push_to_feishu: rng.random_bool(0.3), // 30%概率推送到飞书
        execution_logs,
        last_execution_result,
    }
}

/// Todo统计信息
#[derive(Debug, Clone)]
pub struct TodoStatistics {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub todo: usize,
    pub cancelled: usize,
    pub overdue: usize,
}
