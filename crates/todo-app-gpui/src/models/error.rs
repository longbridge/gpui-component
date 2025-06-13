use std::fmt;

/// Todo应用的错误类型
#[derive(Debug, Clone)]
pub enum TodoError {
    /// 数据验证错误
    ValidationError(String),
    /// 存储错误
    StorageError(String),
    /// 资源未找到错误
    NotFound(String),
    /// ID冲突错误
    DuplicateId(u32),
    /// 通用错误
    Generic(String),
}

impl fmt::Display for TodoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TodoError::ValidationError(msg) => write!(f, "验证错误: {}", msg),
            TodoError::StorageError(msg) => write!(f, "存储错误: {}", msg),
            TodoError::NotFound(msg) => write!(f, "未找到: {}", msg),
            TodoError::DuplicateId(id) => write!(f, "重复的ID: {}", id),
            TodoError::Generic(msg) => write!(f, "错误: {}", msg),
        }
    }
}

impl std::error::Error for TodoError {}

/// Todo应用的结果类型
pub type TodoResult<T> = Result<T, TodoError>;
