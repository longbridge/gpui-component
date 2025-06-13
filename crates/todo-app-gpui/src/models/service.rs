use std::sync::Arc;

use super::error::{TodoError, TodoResult};
use super::repository::TodoRepository;
use super::todo_item::{TodoItem, TodoPriority};

/// Todo业务服务，处理所有业务逻辑
pub struct TodoService {
    repository: Arc<dyn TodoRepository>,
}

impl TodoService {
    pub fn new(repository: Arc<dyn TodoRepository>) -> Self {
        Self { repository }
    }

    /// 创建新的Todo项
    pub fn create_todo(&self, title: String) -> TodoResult<TodoItem> {
        self.create_todo_with_details(title, None, TodoPriority::default())
    }

    /// 创建带详细信息的Todo项
    pub fn create_todo_with_details(
        &self,
        title: String,
        description: Option<String>,
        priority: TodoPriority,
    ) -> TodoResult<TodoItem> {
        let id = self.repository.next_id()?;
        let todo = TodoItem::new_with_details(id, title, description, priority)?;

        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 获取所有Todo项
    pub fn get_all_todos(&self) -> TodoResult<Vec<TodoItem>> {
        self.repository.find_all()
    }

    /// 根据ID获取Todo项
    pub fn get_todo_by_id(&self, id: u32) -> TodoResult<TodoItem> {
        self.repository
            .find_by_id(id)?
            .ok_or_else(|| TodoError::NotFound(format!("未找到ID为{}的Todo项", id)))
    }

    /// 更新Todo项标题
    pub fn update_todo_title(&self, id: u32, new_title: String) -> TodoResult<TodoItem> {
        let mut todo = self.get_todo_by_id(id)?;
        todo.update_title(new_title)?;
        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 更新Todo项描述
    pub fn update_todo_description(
        &self,
        id: u32,
        new_description: Option<String>,
    ) -> TodoResult<TodoItem> {
        let mut todo = self.get_todo_by_id(id)?;
        todo.update_description(new_description)?;
        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 更新Todo项优先级
    pub fn update_todo_priority(
        &self,
        id: u32,
        new_priority: TodoPriority,
    ) -> TodoResult<TodoItem> {
        let mut todo = self.get_todo_by_id(id)?;
        todo.update_priority(new_priority)?;
        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 标记Todo项为已完成
    pub fn mark_todo_completed(&self, id: u32) -> TodoResult<TodoItem> {
        let mut todo = self.get_todo_by_id(id)?;
        todo.mark_completed()?;
        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 标记Todo项为未完成
    pub fn mark_todo_incomplete(&self, id: u32) -> TodoResult<TodoItem> {
        let mut todo = self.get_todo_by_id(id)?;
        todo.mark_incomplete()?;
        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 切换Todo项的完成状态
    pub fn toggle_todo_completion(&self, id: u32) -> TodoResult<TodoItem> {
        let mut todo = self.get_todo_by_id(id)?;
        todo.toggle()?;
        self.repository.save(todo.clone())?;
        Ok(todo)
    }

    /// 删除Todo项
    pub fn delete_todo(&self, id: u32) -> TodoResult<bool> {
        self.repository.delete_by_id(id)
    }

    /// 获取已完成的Todo项
    pub fn get_completed_todos(&self) -> TodoResult<Vec<TodoItem>> {
        self.repository.find_completed()
    }

    /// 获取未完成的Todo项
    pub fn get_pending_todos(&self) -> TodoResult<Vec<TodoItem>> {
        self.repository.find_pending()
    }

    /// 根据优先级获取Todo项
    pub fn get_todos_by_priority(&self, priority: TodoPriority) -> TodoResult<Vec<TodoItem>> {
        self.repository.find_by_priority(priority)
    }

    /// 获取高优先级的未完成Todo项
    pub fn get_high_priority_pending_todos(&self) -> TodoResult<Vec<TodoItem>> {
        let pending_todos = self.get_pending_todos()?;
        let high_priority_todos = pending_todos
            .into_iter()
            .filter(|todo| matches!(todo.priority, TodoPriority::High | TodoPriority::Urgent))
            .collect();
        Ok(high_priority_todos)
    }

    /// 获取Todo统计信息
    pub fn get_todo_stats(&self) -> TodoResult<TodoStats> {
        let all_todos = self.get_all_todos()?;
        let completed_count = all_todos.iter().filter(|todo| todo.completed).count();
        let pending_count = all_todos.len() - completed_count;

        let priority_counts = all_todos
            .iter()
            .fold(PriorityStats::default(), |mut stats, todo| {
                match todo.priority {
                    TodoPriority::Low => stats.low += 1,
                    TodoPriority::Medium => stats.medium += 1,
                    TodoPriority::High => stats.high += 1,
                    TodoPriority::Urgent => stats.urgent += 1,
                }
                stats
            });

        Ok(TodoStats {
            total_count: all_todos.len(),
            completed_count,
            pending_count,
            priority_stats: priority_counts,
        })
    }

    /// 清空所有Todo项
    pub fn clear_all_todos(&self) -> TodoResult<()> {
        self.repository.clear()
    }

    /// 批量操作：标记多个Todo项为已完成
    pub fn mark_multiple_completed(&self, ids: Vec<u32>) -> TodoResult<Vec<TodoResult<TodoItem>>> {
        let results = ids
            .into_iter()
            .map(|id| self.mark_todo_completed(id))
            .collect();
        Ok(results)
    }

    /// 批量操作：删除多个Todo项
    pub fn delete_multiple_todos(&self, ids: Vec<u32>) -> TodoResult<Vec<TodoResult<bool>>> {
        let results = ids.into_iter().map(|id| self.delete_todo(id)).collect();
        Ok(results)
    }

    /// 搜索Todo项
    pub fn search_todos(&self, query: &str) -> TodoResult<Vec<TodoItem>> {
        let all_todos = self.get_all_todos()?;
        let query_lower = query.to_lowercase();

        let filtered_todos = all_todos
            .into_iter()
            .filter(|todo| {
                todo.title.to_lowercase().contains(&query_lower)
                    || todo
                        .description
                        .as_ref()
                        .map(|desc| desc.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect();

        Ok(filtered_todos)
    }
}

/// Todo统计信息
#[derive(Debug, Clone)]
pub struct TodoStats {
    pub total_count: usize,
    pub completed_count: usize,
    pub pending_count: usize,
    pub priority_stats: PriorityStats,
}

/// 优先级统计信息
#[derive(Debug, Clone, Default)]
pub struct PriorityStats {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub urgent: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::repository::InMemoryTodoRepository;

    fn create_test_service() -> TodoService {
        let repository = Arc::new(InMemoryTodoRepository::new());
        TodoService::new(repository)
    }

    #[test]
    fn test_create_todo() {
        let service = create_test_service();
        let result = service.create_todo("测试任务".to_string());

        assert!(result.is_ok());
        let todo = result.unwrap();
        assert_eq!(todo.title, "测试任务");
        assert!(!todo.completed);
        assert_eq!(todo.id, 1);
    }

    #[test]
    fn test_toggle_todo_completion() {
        let service = create_test_service();
        let todo = service.create_todo("测试任务".to_string()).unwrap();

        // 切换为已完成
        let result = service.toggle_todo_completion(todo.id);
        assert!(result.is_ok());
        let updated_todo = result.unwrap();
        assert!(updated_todo.completed);

        // 切换为未完成
        let result = service.toggle_todo_completion(todo.id);
        assert!(result.is_ok());
        let updated_todo = result.unwrap();
        assert!(!updated_todo.completed);
    }

    #[test]
    fn test_search_todos() {
        let service = create_test_service();
        service.create_todo("买牛奶".to_string()).unwrap();
        service.create_todo("写代码".to_string()).unwrap();
        service.create_todo("买面包".to_string()).unwrap();

        let results = service.search_todos("买").unwrap();
        assert_eq!(results.len(), 2);

        let results = service.search_todos("代码").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "写代码");
    }
}
