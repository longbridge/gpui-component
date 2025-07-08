use chrono::Local;
use std::env;
use sysinfo::System;

use crate::backoffice::agentic::ToolDefinition;

fn get_os_info() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = System::name().unwrap_or_else(|| "Unknown OS".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown Version".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown Kernel".to_string());
    format!("{} {} (Kernel: {})", os_name, os_version, kernel_version)
}

fn get_hostname() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();

    System::host_name()
        .unwrap_or_else(|| "Unknown Hostname".to_string())
        .to_string()
}

fn get_current_datetime() -> String {
    Local::now().to_rfc3339()
}

fn get_available_memory() -> String {
    let mut sys = System::new_all();
    sys.refresh_memory();
    let total_memory = sys.total_memory();
    let available_memory = sys.available_memory();
    format!(
        "{:.2} GB / {:.2} GB available",
        available_memory as f64 / (1024.0 * 1024.0 * 1024.0),
        total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
    )
}

fn get_working_directory() -> String {
    env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "Unknown Working Directory".to_string())
}

fn get_home_directory() -> String {
    homedir::my_home().map_or("Unknown Home Directory".to_string(), |path| {
        path.map_or("Unknown Home Directory".to_string(), |path| {
            path.display().to_string()
        })
    })
}

fn get_user_locale() -> String {
    env::var("LANG")
        .or_else(|_| env::var("LC_ALL"))
        .or_else(|_| env::var("LC_MESSAGES"))
        .unwrap_or_else(|_| "Unknown Locale (or not set in env)".to_string())
}

fn get_locate_zone() -> String {
    // 优先取TZ、TIMEZONE等环境变量，否则用chrono的本地时区偏移
    if let Ok(tz) = std::env::var("TZ") {
        return tz;
    }
    if let Ok(tz) = std::env::var("TIMEZONE") {
        return tz;
    }
    let offset = Local::now().offset().to_string();
    format!("Local Offset: {}", offset)
}

pub fn default_prompt() -> String {
    let template = include_str!("default_prompt.md");
    template
        .replace("{{ OS_INFO }}", &get_os_info())
        .replace("{{ HOST_NAME }}", &get_hostname())
        .replace("{{ LOCATE_ZONE }}", &get_locate_zone())
        .replace("{{ CURRENT_DATETIME }}", &get_current_datetime())
        .replace("{{ PYTHON_VERSION }}", "N/A")
        .replace("{{ NODE_VERSION }}", "N/A")
        .replace("{{ AVAILABLE_MEMORY }}", &get_available_memory())
        .replace("{{ WORKING_DIRECTORY }}", &get_working_directory())
        .replace("{{ HOME_DIRECTORY }}", &get_home_directory())
        .replace("{{ USER_LOCALE }}", &get_user_locale())
        .replace("{{ APPLICATION_INFO }}", "xTo-Do | Agentic AI")
}

pub fn prompt_with_tools(tools: Vec<ToolDefinition>) -> String {
    const USER_SYSTEM_PROMPT: &str =
        "You are an assistant, using known tools to help him complete tasks.";
    prompt_with_user_system_prompt(tools, USER_SYSTEM_PROMPT)
}

pub fn prompt_with_user_system_prompt<S: AsRef<str>>(
    tools: Vec<ToolDefinition>,
    user_system_prompt: S,
) -> String {
    let tools_str = tools
        .into_iter()
        .fold(Vec::new(), |mut acc, tool| {
            let tool_entry = format!(
                r#"
<tool>
    <name>{}</name>
    <description>{}</description>
    <arguments>{}</arguments>
</tool>
"#,
                &tool.name, tool.description, &tool.parameters
            );
            acc.push(tool_entry);
            acc
        })
        .join("\n");
    let available_tools = format!(
        r#"
<tools>
    {}
</tools>
        "#,
        tools_str
    );

    let system_prompt_template = include_str!("system_prompt_with_tools.md");
    system_prompt_template
        .replace("{{ OS_INFO }}", &get_os_info())
        .replace("{{ HOST_NAME }}", &get_hostname())
        .replace("{{ LOCATE_ZONE }}", &get_locate_zone())
        .replace("{{ CURRENT_DATETIME }}", &get_current_datetime())
        .replace("{{ PYTHON_VERSION }}", "N/A")
        .replace("{{ NODE_VERSION }}", "N/A")
        .replace("{{ AVAILABLE_MEMORY }}", &get_available_memory())
        .replace("{{ WORKING_DIRECTORY }}", &get_working_directory())
        .replace("{{ HOME_DIRECTORY }}", &get_home_directory())
        .replace("{{ USER_LOCALE }}", &get_user_locale())
        .replace("{{ APPLICATION_INFO }}", "xTo-Do | Agentic AI")
        .replace(
            "{{ TOOL_USE_EXAMPLES }}",
            include_str!("tool_use_examples.md"),
        )
        .replace("{{ AVAILABLE_TOOLS }}", &available_tools)
        .replace("{{ USER_SYSTEM_PROMPT }}", user_system_prompt.as_ref())
}
