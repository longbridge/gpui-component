#[derive(Debug, thiserror::Error)]
pub enum AppVisibilityError {
    #[error("当前平台暂不支持主窗口显隐控制")]
    UnsupportedPlatform,
    #[error("主窗口句柄尚未注册")]
    MainWindowHandleMissing,
    #[error("当前平台窗口句柄类型不受支持")]
    UnsupportedWindowHandle,
    #[error("macOS 激活观察器类初始化失败")]
    ObserverClassInit,
    #[error("macOS 激活观察器实例初始化失败")]
    ObserverInstanceInit,
}
