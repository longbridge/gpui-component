use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::error::{TodoError, TodoResult};
use super::todo_item::{TodoItem, TodoPriority};

/// Todo数据仓库trait，定义数据存储接口
pub trait TodoRepository: Send + Sync {
    /// 保存单个Todo项
    fn save(&self, todo: TodoItem) -> TodoResult<()>;

    /// 根据ID查找Todo项
    fn find_by_id(&self, id: u32) -> TodoResult<Option<TodoItem>>;

    /// 获取所有Todo项
    fn find_all(&self) -> TodoResult<Vec<TodoItem>>;

    /// 根据ID删除Todo项
    fn delete_by_id(&self, id: u32) -> TodoResult<bool>;

    /// 获取已完成的Todo项
    fn find_completed(&self) -> TodoResult<Vec<TodoItem>>;

    /// 获取未完成的Todo项
    fn find_pending(&self) -> TodoResult<Vec<TodoItem>>;

    /// 根据优先级查找Todo项
    fn find_by_priority(&self, priority: TodoPriority) -> TodoResult<Vec<TodoItem>>;

    /// 清空所有数据
    fn clear(&self) -> TodoResult<()>;

    /// 获取下一个可用的ID
    fn next_id(&self) -> TodoResult<u32>;
}

/// 内存中的Todo仓库实现
#[derive(Debug)]
pub struct InMemoryTodoRepository {
    data: Arc<RwLock<HashMap<u32, TodoItem>>>,
    next_id: Arc<RwLock<u32>>,
}

impl InMemoryTodoRepository {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }
}

impl Default for InMemoryTodoRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoRepository for InMemoryTodoRepository {
    fn save(&self, todo: TodoItem) -> TodoResult<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| TodoError::StorageError(format!("获取写锁失败: {}", e)))?;

        data.insert(todo.id, todo);
        Ok(())
    }

    fn find_by_id(&self, id: u32) -> TodoResult<Option<TodoItem>> {
        let data = self
            .data
            .read()
            .map_err(|e| TodoError::StorageError(format!("获取读锁失败: {}", e)))?;

        Ok(data.get(&id).cloned())
    }

    fn find_all(&self) -> TodoResult<Vec<TodoItem>> {
        let data = self
            .data
            .read()
            .map_err(|e| TodoError::StorageError(format!("获取读锁失败: {}", e)))?;

        let mut todos: Vec<TodoItem> = data.values().cloned().collect();
        // 按创建时间排序
        todos.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(todos)
    }

    fn delete_by_id(&self, id: u32) -> TodoResult<bool> {
        let mut data = self
            .data
            .write()
            .map_err(|e| TodoError::StorageError(format!("获取写锁失败: {}", e)))?;

        Ok(data.remove(&id).is_some())
    }

    fn find_completed(&self) -> TodoResult<Vec<TodoItem>> {
        let data = self
            .data
            .read()
            .map_err(|e| TodoError::StorageError(format!("获取读锁失败: {}", e)))?;

        let mut todos: Vec<TodoItem> = data
            .values()
            .filter(|todo| todo.completed)
            .cloned()
            .collect();
        todos.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));
        Ok(todos)
    }

    fn find_pending(&self) -> TodoResult<Vec<TodoItem>> {
        let data = self
            .data
            .read()
            .map_err(|e| TodoError::StorageError(format!("获取读锁失败: {}", e)))?;

        let mut todos: Vec<TodoItem> = data
            .values()
            .filter(|todo| !todo.completed)
            .cloned()
            .collect();
        // 按优先级和创建时间排序
        todos.sort_by(|a, b| {
            b.priority_value()
                .cmp(&a.priority_value())
                .then(a.created_at.cmp(&b.created_at))
        });
        Ok(todos)
    }

    fn find_by_priority(&self, priority: TodoPriority) -> TodoResult<Vec<TodoItem>> {
        let data = self
            .data
            .read()
            .map_err(|e| TodoError::StorageError(format!("获取读锁失败: {}", e)))?;

        let mut todos: Vec<TodoItem> = data
            .values()
            .filter(|todo| todo.priority == priority)
            .cloned()
            .collect();
        todos.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(todos)
    }

    fn clear(&self) -> TodoResult<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| TodoError::StorageError(format!("获取写锁失败: {}", e)))?;

        data.clear();

        let mut next_id = self
            .next_id
            .write()
            .map_err(|e| TodoError::StorageError(format!("获取写锁失败: {}", e)))?;
        *next_id = 1;

        Ok(())
    }

    fn next_id(&self) -> TodoResult<u32> {
        let mut next_id = self
            .next_id
            .write()
            .map_err(|e| TodoError::StorageError(format!("获取写锁失败: {}", e)))?;

        let id = *next_id;
        *next_id += 1;
        Ok(id)
    }
}

/// 基于文件的Todo仓库实现
#[derive(Debug)]
pub struct FileTodoRepository {
    memory_repo: InMemoryTodoRepository,
    file_path: PathBuf,
}

impl FileTodoRepository {
    pub fn new(file_path: PathBuf) -> TodoResult<Self> {
        let repo = Self {
            memory_repo: InMemoryTodoRepository::new(),
            file_path,
        };

        // 尝试从文件加载数据
        if let Err(e) = repo.load_from_file() {
            eprintln!("警告: 无法从文件加载数据: {}", e);
        }

        Ok(repo)
    }

    fn load_from_file(&self) -> TodoResult<()> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.file_path)
            .map_err(|e| TodoError::StorageError(format!("读取文件失败: {}", e)))?;

        if content.trim().is_empty() {
            return Ok(());
        }

        let todos: Vec<TodoItem> = serde_json::from_str(&content)
            .map_err(|e| TodoError::StorageError(format!("解析JSON失败: {}", e)))?;

        // 将数据加载到内存仓库
        for todo in todos {
            self.memory_repo.save(todo)?;
        }

        Ok(())
    }

    fn save_to_file(&self) -> TodoResult<()> {
        let todos = self.memory_repo.find_all()?;
        let content = serde_json::to_string_pretty(&todos)
            .map_err(|e| TodoError::StorageError(format!("序列化JSON失败: {}", e)))?;

        // 确保父目录存在
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| TodoError::StorageError(format!("创建目录失败: {}", e)))?;
        }

        fs::write(&self.file_path, content)
            .map_err(|e| TodoError::StorageError(format!("写入文件失败: {}", e)))?;

        Ok(())
    }
}

impl TodoRepository for FileTodoRepository {
    fn save(&self, todo: TodoItem) -> TodoResult<()> {
        self.memory_repo.save(todo)?;
        self.save_to_file()
    }

    fn find_by_id(&self, id: u32) -> TodoResult<Option<TodoItem>> {
        self.memory_repo.find_by_id(id)
    }

    fn find_all(&self) -> TodoResult<Vec<TodoItem>> {
        self.memory_repo.find_all()
    }

    fn delete_by_id(&self, id: u32) -> TodoResult<bool> {
        let result = self.memory_repo.delete_by_id(id)?;
        if result {
            self.save_to_file()?;
        }
        Ok(result)
    }

    fn find_completed(&self) -> TodoResult<Vec<TodoItem>> {
        self.memory_repo.find_completed()
    }

    fn find_pending(&self) -> TodoResult<Vec<TodoItem>> {
        self.memory_repo.find_pending()
    }

    fn find_by_priority(&self, priority: TodoPriority) -> TodoResult<Vec<TodoItem>> {
        self.memory_repo.find_by_priority(priority)
    }

    fn clear(&self) -> TodoResult<()> {
        self.memory_repo.clear()?;
        self.save_to_file()
    }

    fn next_id(&self) -> TodoResult<u32> {
        self.memory_repo.next_id()
    }
}
