use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::models::profile_config_path;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct ProfileData {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub auto_analyze_bio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileManager {
    #[serde(flatten, default)]
    pub profile: ProfileData,
}

impl ProfileManager {
    /// 从文件加载配置
    pub fn load() -> Self {
        let content = std::fs::read_to_string(profile_config_path())
            .map_or("".to_string(), |content| content);

        let manager: ProfileManager =
            serde_yaml::from_str(&content).map_or(ProfileManager::default(), |profile| profile);
        manager
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;

        if let Some(parent) = profile_config_path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(profile_config_path(), content)?;
        Ok(())
    }

    /// 获取个人资料
    pub fn get_profile(&self) -> &ProfileData {
        &self.profile
    }

    /// 更新个人资料
    pub fn update_profile(&mut self, profile: ProfileData) -> &mut Self {
        self.profile = profile;
        self
    }

    /// 更新姓名
    pub fn update_name(&mut self, name: String) -> &mut Self {
        self.profile.name = name;
        self
    }

    /// 更新邮箱
    pub fn update_email(&mut self, email: String) -> anyhow::Result<&mut Self> {
        if !email.is_empty() && !email.contains('@') {
            return Err(anyhow::anyhow!("Invalid email format"));
        }
        self.profile.email = email;
        Ok(self)
    }

    /// 更新电话
    pub fn update_phone(&mut self, phone: String) -> &mut Self {
        self.profile.phone = phone;
        self
    }

    /// 更新个人简介
    pub fn update_bio(&mut self, bio: String) -> &mut Self {
        self.profile.bio = bio;
        self
    }

    /// 更新主题
    pub fn update_theme(&mut self, theme: String) -> anyhow::Result<&mut Self> {
        let valid_themes = ["light", "dark", "auto"];
        if !theme.is_empty() && !valid_themes.contains(&theme.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid theme. Valid themes: {:?}",
                valid_themes
            ));
        }
        self.profile.theme = theme;
        Ok(self)
    }

    /// 更新语言
    pub fn update_language(&mut self, language: String) -> &mut Self {
        self.profile.language = language;
        self
    }

    /// 更新部门
    pub fn update_department(&mut self, department: String) -> &mut Self {
        self.profile.department = department;
        self
    }

    /// 更新自动分析简介
    pub fn update_auto_analyze_bio(&mut self, auto_analyze_bio: bool) -> &mut Self {
        self.profile.auto_analyze_bio = auto_analyze_bio;
        self
    }

    /// 重置个人资料到默认值
    pub fn reset_profile(&mut self) -> &mut Self {
        self.profile = ProfileData::default();
        self
    }

    /// 验证个人资料数据
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.profile.email.is_empty() && !self.profile.email.contains('@') {
            return Err(anyhow::anyhow!("Invalid email format"));
        }

        if self.profile.name.is_empty() {
            return Err(anyhow::anyhow!("name is required"));
        }

        Ok(())
    }

    /// 批量更新多个字段
    pub fn batch_update(&mut self, updates: ProfileUpdates) -> anyhow::Result<()> {
        if let Some(name) = updates.name {
            self.update_name(name);
        }
        if let Some(email) = updates.email {
            self.update_email(email)?;
        }
        if let Some(phone) = updates.phone {
            self.update_phone(phone);
        }
        if let Some(bio) = updates.bio {
            self.update_bio(bio);
        }
        if let Some(theme) = updates.theme {
            self.update_theme(theme)?;
        }
        if let Some(language) = updates.language {
            self.update_language(language);
        }
        if let Some(department) = updates.department {
            self.update_department(department);
        }
        if let Some(auto_analyze_bio) = updates.auto_analyze_bio {
            self.update_auto_analyze_bio(auto_analyze_bio);
        }
        Ok(())
    }

    /// 检查个人资料是否为空
    pub fn is_empty(&self) -> bool {
        self.profile.name.is_empty()
            && self.profile.email.is_empty()
            && self.profile.phone.is_empty()
            && self.profile.bio.is_empty()
            && self.profile.theme.is_empty()
            && self.profile.language.is_empty()
            && self.profile.department.is_empty()
            && !self.profile.auto_analyze_bio
    }
}

/// 用于批量更新的结构体
#[derive(Debug, Default)]
pub struct ProfileUpdates {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub bio: Option<String>,
    pub username: Option<String>,
    pub theme: Option<String>,
    pub language: Option<String>,
    pub department: Option<String>,
    pub auto_analyze_bio: Option<bool>,
}

impl ProfileUpdates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn phone(mut self, phone: String) -> Self {
        self.phone = Some(phone);
        self
    }

    pub fn bio(mut self, bio: String) -> Self {
        self.bio = Some(bio);
        self
    }

    pub fn username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn theme(mut self, theme: String) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn department(mut self, department: String) -> Self {
        self.department = Some(department);
        self
    }

    pub fn auto_analyze_bio(mut self, auto_analyze_bio: bool) -> Self {
        self.auto_analyze_bio = Some(auto_analyze_bio);
        self
    }
}
