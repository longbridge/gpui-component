use crate::{
    backoffice::{builtin::ticker::Tick, cross_runtime::CrossRuntimeBridge},
    xbus::Subscription,
};
use actix::prelude::*;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

// 任务执行事件，用于广播给 LLM-Agent
#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct JobExecutionEvent {
    pub job_id: String,
    pub job_name: String,
    pub instructions: String,
    pub refid: Option<String>, // 来自哪个 LLM-Agent
    pub execution_time: DateTime<Utc>,
    pub attempt: u8,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: Schedule,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
    pub enabled: bool,
    pub instructions: String,  // 任务执行指令
    pub refid: Option<String>, // 来自哪个 LLM-Agent
    pub metadata: HashMap<String, String>,
}

impl CronJob {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        cron_expr: &str,
        instructions: impl Into<String>,
        refid: Option<String>,
    ) -> anyhow::Result<Self> {
        let schedule = Schedule::from_str(cron_expr)?;
        let next_run = schedule
            .upcoming(Utc)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid cron expression"))?;

        Ok(CronJob {
            id: id.into(),
            name: name.into(),
            schedule,
            last_run: None,
            next_run,
            enabled: true,
            instructions: instructions.into(),
            refid,
            metadata: HashMap::new(),
        })
    }

    pub fn should_run(&self, now: DateTime<Utc>) -> bool {
        self.enabled && now >= self.next_run
    }

    pub fn update_next_run(&mut self) {
        if let Some(next) = self.schedule.upcoming(Utc).next() {
            self.next_run = next;
        }
    }

    // 执行任务：广播任务执行事件给 LLM-Agent
    pub fn execute(&mut self) {
        let now = Utc::now();

        let event = JobExecutionEvent {
            job_id: self.id.clone(),
            job_name: self.name.clone(),
            instructions: self.instructions.clone(),
            refid: self.refid.clone(),
            execution_time: now,
            attempt: 1,
            metadata: self.metadata.clone(),
        };

        // 广播任务执行事件
        CrossRuntimeBridge::global().emit(event);
        self.last_run = Some(now);
        self.update_next_run();

        tracing::info!(
            "Cron job '{}' ({}) from agent {:?} scheduled for execution at {}",
            self.name,
            self.id,
            self.refid,
            now
        );
    }
}

// 为了能在 HashMap 中使用，我们需要一个简化的结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCronJob {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
    pub enabled: bool,
    pub instructions: String,
    pub refid: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub struct AddJob {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub instructions: String,
    pub refid: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub struct RemoveJob {
    pub id: String,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub struct EnableJob {
    pub id: String,
    pub enabled: bool,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "Vec<StoredCronJob>")]
pub struct ListJobs;

#[derive(Default, Debug)]
pub struct JobScheduler {
    subscription: Option<Subscription>,
    jobs: HashMap<String, CronJob>,
}

impl JobScheduler {
    pub fn global() -> Addr<Self> {
        Self::from_registry()
    }

    fn check_and_run_jobs(&mut self) {
        let now = Utc::now();
        let mut jobs_to_run = Vec::new();

        // 收集需要运行的任务
        for (id, job) in &self.jobs {
            if job.should_run(now) {
                jobs_to_run.push(id.clone());
            }
        }

        // 执行任务（广播事件）
        for job_id in jobs_to_run {
            if let Some(job) = self.jobs.get_mut(&job_id) {
                job.execute();
            }
        }
    }

    // 预设一些常用的 cron 表达式示例
    pub fn add_example_jobs(&mut self) {
        // 每分钟执行一次
        let _ = self.add_job_internal(
            "every_minute",
            "Every Minute Test",
            "0 * * * * *",
            "执行每分钟的系统状态检查：记录当前时间、检查内存使用率、CPU负载和磁盘空间",
            Some("system_monitor".to_string()),
            HashMap::new(),
        );

        // 每小时执行一次
        let _ = self.add_job_internal(
            "hourly",
            "Hourly Task",
            "0 0 * * * *",
            "执行每小时的系统维护：清理临时缓存、检查日志文件大小、监控服务状态",
            Some("system_maintenance".to_string()),
            HashMap::new(),
        );

        // 每天凌晨 2 点执行
        let _ = self.add_job_internal(
            "daily_2am",
            "Daily Cleanup",
            "0 0 2 * * *",
            "执行每日系统清理：删除7天前的临时文件、压缩旧日志、清理缓存目录",
            Some("cleanup_agent".to_string()),
            {
                let mut meta = HashMap::new();
                meta.insert("cleanup_path".to_string(), "/tmp".to_string());
                meta.insert("days_old".to_string(), "7".to_string());
                meta
            },
        );

        // 每周一上午 9 点执行
        let _ = self.add_job_internal(
            "weekly_monday",
            "Weekly Report",
            "0 0 9 * * MON",
            "生成系统周报：统计过去一周的系统性能数据、分析错误日志、生成使用报告并发送给管理员",
            Some("report_agent".to_string()),
            {
                let mut meta = HashMap::new();
                meta.insert("report_type".to_string(), "system_status".to_string());
                meta.insert("recipients".to_string(), "admin@example.com".to_string());
                meta
            },
        );
    }

    fn add_job_internal(
        &mut self,
        id: &str,
        name: &str,
        cron_expr: &str,
        instructions: &str,
        refid: Option<String>,
        metadata: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let mut job = CronJob::new(id, name, cron_expr, instructions, refid)?;
        job.metadata = metadata;

        self.jobs.insert(id.to_string(), job);
        tracing::info!("Added cron job '{}' ({}): {}", name, id, cron_expr);
        Ok(())
    }
}

impl Handler<Tick> for JobScheduler {
    type Result = ();

    fn handle(&mut self, msg: Tick, _ctx: &mut Self::Context) -> Self::Result {
        tracing::debug!("JobScheduler received tick: {}", msg.0);
        self.check_and_run_jobs();
    }
}

impl Handler<AddJob> for JobScheduler {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: AddJob, _ctx: &mut Self::Context) -> Self::Result {
        self.add_job_internal(
            &msg.id,
            &msg.name,
            &msg.cron_expr,
            &msg.instructions,
            msg.refid,
            msg.metadata,
        )
    }
}

impl Handler<RemoveJob> for JobScheduler {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: RemoveJob, _ctx: &mut Self::Context) -> Self::Result {
        if self.jobs.remove(&msg.id).is_some() {
            tracing::info!("Removed cron job: {}", msg.id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Job with id '{}' not found", msg.id))
        }
    }
}

impl Handler<EnableJob> for JobScheduler {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: EnableJob, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(job) = self.jobs.get_mut(&msg.id) {
            job.enabled = msg.enabled;
            tracing::info!(
                "Job '{}' {}",
                msg.id,
                if msg.enabled { "enabled" } else { "disabled" }
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Job with id '{}' not found", msg.id))
        }
    }
}

impl Handler<ListJobs> for JobScheduler {
    type Result = Vec<StoredCronJob>;

    fn handle(&mut self, _msg: ListJobs, _ctx: &mut Self::Context) -> Self::Result {
        self.jobs
            .values()
            .map(|job| StoredCronJob {
                id: job.id.clone(),
                name: job.name.clone(),
                cron_expr: job.schedule.to_string(),
                last_run: job.last_run,
                next_run: job.next_run,
                enabled: job.enabled,
                instructions: job.instructions.clone(),
                refid: job.refid.clone(),
                metadata: job.metadata.clone(),
            })
            .collect()
    }
}

impl Supervised for JobScheduler {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("JobScheduler is restarting");
    }
}

impl SystemService for JobScheduler {}

impl Actor for JobScheduler {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("JobScheduler started");
        let addr = ctx.address();
        self.subscription = Some(CrossRuntimeBridge::global().subscribe(move |tick: &Tick| {
            addr.try_send(tick.clone()).unwrap_or_else(|err| {
                tracing::error!("Failed to send tick to JobScheduler: {}", err);
            })
        }));
        // 添加示例任务
        self.add_example_jobs();
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("JobScheduler stopped");
    }
}

// 使用示例的辅助函数
impl JobScheduler {
    pub async fn add_job_async(
        id: impl Into<String>,
        name: impl Into<String>,
        cron_expr: &str,
        instructions: impl Into<String>,
        refid: Option<String>,
    ) -> anyhow::Result<()> {
        let scheduler = Self::global();
        scheduler
            .send(AddJob {
                id: id.into(),
                name: name.into(),
                cron_expr: cron_expr.to_string(),
                instructions: instructions.into(),
                refid,
                metadata: HashMap::new(),
            })
            .await??;
        Ok(())
    }

    pub async fn add_job_with_metadata_async(
        id: impl Into<String>,
        name: impl Into<String>,
        cron_expr: &str,
        instructions: impl Into<String>,
        refid: Option<String>,
        metadata: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let scheduler = Self::global();
        scheduler
            .send(AddJob {
                id: id.into(),
                name: name.into(),
                cron_expr: cron_expr.to_string(),
                instructions: instructions.into(),
                refid,
                metadata,
            })
            .await??;
        Ok(())
    }

    pub async fn remove_job_async(id: impl Into<String>) -> anyhow::Result<()> {
        let scheduler = Self::global();
        scheduler.send(RemoveJob { id: id.into() }).await??;
        Ok(())
    }

    pub async fn enable_job_async(id: impl Into<String>, enabled: bool) -> anyhow::Result<()> {
        let scheduler = Self::global();
        scheduler
            .send(EnableJob {
                id: id.into(),
                enabled,
            })
            .await??;
        Ok(())
    }

    pub async fn list_jobs_async() -> anyhow::Result<Vec<StoredCronJob>> {
        let scheduler = Self::global();
        let jobs = scheduler.send(ListJobs).await?;
        Ok(jobs)
    }
}
