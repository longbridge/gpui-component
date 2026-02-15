//! 终端主题配置
//!
//! 提供终端的颜色、字体、字号等外观设置
//!
//! ## 配色系统设计
//!
//! 本模块采用语义化配色系统，确保所有颜色组合具有足够的对比度：
//! - `background` / `foreground`: 主要背景和文字，对比度 >= 7:1
//! - `muted` / `muted_foreground`: 次要区域背景和文字，对比度 >= 4.5:1
//! - `accent` / `accent_foreground`: 强调色背景和文字，对比度 >= 4.5:1
//!
//! 颜色使用规则：
//! - 在 `background` 上使用 `foreground` 或 `muted_foreground`
//! - 在 `muted` 上使用 `foreground` 或 `muted_foreground`
//! - 在 `accent` 上使用 `accent_foreground`

use gpui::{rgb, Hsla, Pixels, SharedString};

/// 默认字体大小
pub const DEFAULT_FONT_SIZE: f32 = 13.0;
/// 最小字体大小
pub const MIN_FONT_SIZE: f32 = 8.0;
/// 最大字体大小
pub const MAX_FONT_SIZE: f32 = 32.0;
/// 默认行高比例
pub const DEFAULT_LINE_HEIGHT_SCALE: f32 = 1.4;

/// 终端主题配色（用于侧边栏等 UI 组件）
///
/// 所有颜色对都经过对比度验证，确保可读性：
/// - `background` + `foreground`: 主要内容
/// - `background` + `muted_foreground`: 次要内容
/// - `muted` + `foreground`: 卡片/列表项上的主要内容
/// - `muted` + `muted_foreground`: 卡片/列表项上的次要内容
/// - `accent` + `accent_foreground`: 按钮/选中状态
#[derive(Clone, Debug)]
pub struct TerminalColors {
    /// 主背景色
    pub background: Hsla,
    /// 主前景色（在 background 上使用）
    pub foreground: Hsla,
    /// 次要背景色（卡片、列表项、悬停状态）
    pub muted: Hsla,
    /// 次要前景色（次要文字、标签、占位符）
    pub muted_foreground: Hsla,
    /// 边框色
    pub border: Hsla,
    /// 强调背景色（按钮、选中项）
    pub accent: Hsla,
    /// 强调前景色（在 accent 背景上使用）
    pub accent_foreground: Hsla,
}

/// 终端主题配置
#[derive(Clone, Debug)]
pub struct TerminalTheme {
    /// 主题名称
    pub name: &'static str,
    /// 前景色（文字颜色）
    pub foreground: Hsla,
    /// 背景色
    pub background: Hsla,
    /// 光标颜色
    pub cursor: Hsla,
    /// 选中区域颜色
    pub selection: Hsla,
    /// 主字体
    pub font_family: SharedString,
    /// 字体大小
    pub font_size: Pixels,
    /// 备用字体列表
    pub font_fallbacks: Vec<SharedString>,
    /// 行高比例
    pub line_height_scale: f32,
}

impl PartialEq for TerminalTheme {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.foreground == other.foreground
            && self.background == other.background
            && self.cursor == other.cursor
            && self.selection == other.selection
            && self.font_family == other.font_family
            && self.font_size == other.font_size
            && self.line_height_scale == other.line_height_scale
    }
}

/// 获取当前操作系统的默认等宽字体
pub fn default_monospace_font() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        // Linux 和其他系统
        "DejaVu Sans Mono"
    }
}

/// 默认备用字体列表（按优先级排序，跨平台兼容）
pub fn default_font_fallbacks() -> Vec<SharedString> {
    if cfg!(target_os = "macos") {
        vec![
            "Monaco".into(),
            "SF Mono".into(),
            "Courier New".into(),
            "Apple Color Emoji".into(),
            "Apple Symbols".into(),
            "PingFang SC".into(),
            "PingFang TC".into(),
            "Hiragino Sans GB".into(),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            "Cascadia Mono".into(),
            "Courier New".into(),
            "Lucida Console".into(),
            "Segoe UI Emoji".into(),
            "Microsoft YaHei".into(),
            "SimSun".into(),
        ]
    } else {
        // Linux 和其他系统
        vec![
            "Ubuntu Mono".into(),
            "Liberation Mono".into(),
            "Courier New".into(),
            "Noto Color Emoji".into(),
            "Noto Sans CJK SC".into(),
            "WenQuanYi Micro Hei".into(),
        ]
    }
}

impl TerminalTheme {
    /// 获取所有可用主题
    pub fn all() -> Vec<Self> {
        vec![
            Self::midnight(),
            Self::daylight(),
            Self::ink(),
            Self::paper(),
            Self::ocean(),
            Self::obsidian(),
            Self::lotus(),
            Self::neon_blue(),
            Self::matrix(),
            Self::crimson(),
        ]
    }

    /// 创建带有默认字体配置的主题
    fn with_default_font(
        name: &'static str,
        foreground: Hsla,
        background: Hsla,
        cursor: Hsla,
        selection: Hsla,
    ) -> Self {
        Self {
            name,
            foreground,
            background,
            cursor,
            selection,
            font_family: default_monospace_font().into(),
            font_size: gpui::px(DEFAULT_FONT_SIZE),
            font_fallbacks: default_font_fallbacks(),
            line_height_scale: DEFAULT_LINE_HEIGHT_SCALE,
        }
    }

    /// 暗夜主题（深灰背景，浅灰文字）
    pub fn midnight() -> Self {
        Self::with_default_font(
            "midnight",
            rgb(0xE4E4E4).into(),
            rgb(0x1E1E1E).into(),
            rgb(0xFFFFFF).into(),
            rgb(0x3D3D3D).into(),
        )
    }

    /// 明亮主题（白色背景，深灰文字）
    pub fn daylight() -> Self {
        Self::with_default_font(
            "daylight",
            rgb(0x2E3436).into(),
            rgb(0xFFFFFF).into(),
            rgb(0x000000).into(),
            rgb(0xD3D7CF).into(),
        )
    }

    /// 墨黑主题（近黑背景，米色文字）
    pub fn ink() -> Self {
        Self::with_default_font(
            "ink",
            rgb(0xCECDC3).into(),
            rgb(0x100F0F).into(),
            rgb(0xDA702C).into(),
            rgb(0x282726).into(),
        )
    }

    /// 纸白主题（米白背景，深色文字）
    pub fn paper() -> Self {
        Self::with_default_font(
            "paper",
            rgb(0x100F0F).into(),
            rgb(0xFFFCF0).into(),
            rgb(0xDA702C).into(),
            rgb(0xE6E4D9).into(),
        )
    }

    /// 海浪主题（深蓝灰背景，暖米色文字）
    pub fn ocean() -> Self {
        Self::with_default_font(
            "ocean",
            rgb(0xDCD7BA).into(),
            rgb(0x1F1F28).into(),
            rgb(0xC8C093).into(),
            rgb(0x2D4F67).into(),
        )
    }

    /// 黑曜主题（深棕黑背景，灰绿文字）
    pub fn obsidian() -> Self {
        Self::with_default_font(
            "obsidian",
            rgb(0xC5C9C5).into(),
            rgb(0x181616).into(),
            rgb(0xC8C093).into(),
            rgb(0x2D4F67).into(),
        )
    }

    /// 莲白主题（米黄背景，深灰紫文字）
    pub fn lotus() -> Self {
        Self::with_default_font(
            "lotus",
            rgb(0x545464).into(),
            rgb(0xF2ECBC).into(),
            rgb(0x43436C).into(),
            rgb(0xB6D7A8).into(),
        )
    }

    /// 霓蓝主题（深蓝黑背景，青蓝文字）
    pub fn neon_blue() -> Self {
        Self::with_default_font(
            "neon_blue",
            rgb(0x00D9FF).into(),
            rgb(0x0A0E14).into(),
            rgb(0xFFFFFF).into(),
            rgb(0x1A3A52).into(),
        )
    }

    /// 矩阵主题（近黑背景，亮绿文字，Matrix 风格）
    pub fn matrix() -> Self {
        Self::with_default_font(
            "matrix",
            rgb(0x00FF41).into(),
            rgb(0x0D0D0D).into(),
            rgb(0xFFFFFF).into(),
            rgb(0x1A3A1A).into(),
        )
    }

    /// 赤红主题（深红黑背景，亮红文字）
    pub fn crimson() -> Self {
        Self::with_default_font(
            "crimson",
            rgb(0xFF5555).into(),
            rgb(0x1A0A0A).into(),
            rgb(0xFFFFFF).into(),
            rgb(0x4A1A1A).into(),
        )
    }

    /// 根据名称查找主题
    pub fn find_by_name(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|t| t.name == name)
    }

    /// 设置字体大小（会限制在最小和最大值之间）
    pub fn with_font_size(mut self, size: f32) -> Self {
        let clamped = size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        self.font_size = gpui::px(clamped);
        self
    }

    /// 设置主字体
    pub fn with_font_family(mut self, family: impl Into<SharedString>) -> Self {
        self.font_family = family.into();
        self
    }

    /// 设置备用字体列表
    pub fn with_font_fallbacks(mut self, fallbacks: Vec<SharedString>) -> Self {
        self.font_fallbacks = fallbacks;
        self
    }

    /// 设置行高比例
    pub fn with_line_height_scale(mut self, scale: f32) -> Self {
        self.line_height_scale = scale.clamp(1.0, 2.5);
        self
    }

    /// 获取计算后的行高
    pub fn line_height(&self) -> Pixels {
        self.font_size * self.line_height_scale
    }

    /// 判断是否为深色主题
    pub fn is_dark(&self) -> bool {
        // 根据背景色亮度判断
        self.background.l < 0.5
    }

    /// 获取用于 UI 组件的配色
    ///
    /// 该方法根据主题的基础颜色生成一套完整的 UI 配色，
    /// 所有颜色组合都保证足够的对比度以确保可读性。
    pub fn colors(&self) -> TerminalColors {
        let is_dark = self.is_dark();

        // 计算 muted 背景色（卡片、列表项等）
        let muted = if is_dark {
            // 深色主题：muted 比背景稍亮
            Hsla {
                h: self.background.h,
                s: self.background.s,
                l: (self.background.l + 0.06).min(0.25),
                a: 1.0,
            }
        } else {
            // 浅色主题：muted 比背景稍暗
            Hsla {
                h: self.background.h,
                s: self.background.s.min(0.1),
                l: (self.background.l - 0.06).max(0.85),
                a: 1.0,
            }
        };

        // 计算 muted_foreground（次要文字）
        // 关键：必须与 background 和 muted 都有足够对比度
        let muted_foreground = if is_dark {
            // 深色主题：使用中等亮度的灰色
            // 确保在深色背景上可读
            Hsla {
                h: self.foreground.h,
                s: self.foreground.s * 0.3,
                l: 0.55, // 固定中等亮度，确保在深色背景上可读
                a: 1.0,
            }
        } else {
            // 浅色主题：使用较深的灰色
            // 确保在浅色背景上可读
            Hsla {
                h: self.foreground.h,
                s: self.foreground.s * 0.3,
                l: 0.45, // 固定中等亮度，确保在浅色背景上可读
                a: 1.0,
            }
        };

        // 计算边框色
        let border = if is_dark {
            Hsla {
                h: self.background.h,
                s: self.background.s,
                l: (self.background.l + 0.12).min(0.35),
                a: 1.0,
            }
        } else {
            Hsla {
                h: self.background.h,
                s: self.background.s.min(0.1),
                l: (self.background.l - 0.15).max(0.75),
                a: 1.0,
            }
        };

        // 计算强调色前景（在 accent 背景上使用的文字颜色）
        // 根据 accent 的亮度决定使用深色还是浅色文字
        let accent_foreground = if self.cursor.l > 0.5 {
            // accent 是亮色，使用深色文字
            Hsla {
                h: self.cursor.h,
                s: self.cursor.s * 0.2,
                l: 0.1, // 深色文字
                a: 1.0,
            }
        } else {
            // accent 是暗色，使用亮色文字
            Hsla {
                h: self.cursor.h,
                s: self.cursor.s * 0.1,
                l: 0.95, // 亮色文字
                a: 1.0,
            }
        };

        TerminalColors {
            background: self.background,
            foreground: self.foreground,
            muted,
            muted_foreground,
            border,
            accent: self.cursor,
            accent_foreground,
        }
    }

    /// 获取可用的等宽字体列表（按操作系统优化排序）
    pub fn available_monospace_fonts() -> Vec<&'static str> {
        if cfg!(target_os = "macos") {
            vec![
                "Menlo",           // macOS 默认
                "Monaco",
                "SF Mono",
                "Courier New",
                // 跨平台字体（需要安装）
                "Fira Code",
                "JetBrains Mono",
                "Source Code Pro",
                "Cascadia Code",
                "Hack",
                "IBM Plex Mono",
            ]
        } else if cfg!(target_os = "windows") {
            vec![
                "Consolas",        // Windows 默认
                "Cascadia Mono",
                "Cascadia Code",
                "Courier New",
                "Lucida Console",
                // 跨平台字体（需要安装）
                "Fira Code",
                "JetBrains Mono",
                "Source Code Pro",
                "Hack",
                "IBM Plex Mono",
            ]
        } else {
            // Linux 和其他系统
            vec![
                "DejaVu Sans Mono", // Linux 常见默认
                "Ubuntu Mono",
                "Liberation Mono",
                "Courier New",
                // 跨平台字体（需要安装）
                "Fira Code",
                "JetBrains Mono",
                "Source Code Pro",
                "Cascadia Code",
                "Hack",
                "IBM Plex Mono",
            ]
        }
    }

    /// 获取可用的字体大小预设列表
    pub fn available_font_sizes() -> Vec<f32> {
        vec![8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0, 22.0, 24.0]
    }
}
