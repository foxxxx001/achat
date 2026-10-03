//! Runtime locale detection: Windows + China region => Chinese UI messages.

use std::sync::OnceLock;

static IS_CN: OnceLock<bool> = OnceLock::new();

/// Windows AND appears to be in the China region => Chinese UI.
pub fn is_cn() -> bool {
    *IS_CN.get_or_init(|| {
        // 0) config.yaml `language: cn` wins over everything
        if let Some(v) = config_language() {
            return v.eq_ignore_ascii_case("cn") || v.eq_ignore_ascii_case("zh");
        }
        // manual override for testing / forced locale
        if let Ok(v) = std::env::var("ACHAT_LANG") {
            let v = v.to_lowercase();
            if v == "zh" || v == "zh-cn" || v == "cn" {
                return true;
            }
            if v == "en" {
                return false;
            }
        }
        if !cfg!(windows) {
            return false;
        }
        // 1) OS locale (e.g. zh-CN)
        if let Some(locale) = sys_locale::get_locale() {
            let l = locale.to_lowercase();
            if l.starts_with("zh") {
                return true;
            }
        }
        // 2) Environment hints
        for key in ["LANG", "LC_ALL", "LANGUAGE"] {
            if let Ok(v) = std::env::var(key) {
                let v = v.to_lowercase();
                if v.contains("zh_cn") || v.contains("zh-cn") {
                    return true;
                }
            }
        }
        // 3) Time zone hints
        if let Ok(tz) = std::env::var("TZ") {
            let tz = tz.to_lowercase();
            if tz.contains("shanghai")
                || tz.contains("hongkong")
                || tz.contains("chongqing")
                || tz.contains("urumqi")
            {
                return true;
            }
        }
        // 4) UTC+8 local offset
        use chrono::{Local, Timelike};
        let offset_secs = Local::now().offset().local_minus_utc();
        if offset_secs == 8 * 3600 {
            return true;
        }
        false
    })
}

/// Read the top-level `language` value from config.yaml without full parsing.
fn config_language() -> Option<String> {
    let path = crate::config::Config::config_file();
    let content = std::fs::read_to_string(&path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("language:") {
            let v = v.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Chinese help text for `achat --help` when the UI language is cn.
/// (clap's derive help is static English; we override it at runtime.)
pub fn chinese_help() -> String {
    let v = env!("CARGO_PKG_VERSION");
    format!(
        r#"achat {v}
Made by Gary-China

用法: achat [参数] [文本...]

All-in-one LLM CLI 工具（fork 自 aichat，支持 Responses API）。

参数:
  -m, --model <MODEL>              选择 LLM 模型
      --prompt <PROMPT>            使用系统提示词
  -r, --role <ROLE>                选择角色
  -s, --session [<SESSION>]        开始或加入会话
      --empty-session              确保会话为空
      --save-session               将新对话保存到会话
  -a, --agent <AGENT>              启动一个 agent
      --agent-variable <名称 值>   设置 agent 变量
      --rag <RAG>                  启动一个 RAG
      --rebuild-rag                重建 RAG 以同步文档变更
      --macro <MACRO>              执行宏
      --serve [<地址>]             启动 LLM API 与 Web 服务
  -n, --name <NAME>                按名称选择客户端并 GET 其 /v1/models
      --init                       交互式初始化/扩展配置文件
  -qn, --list-name                列出 config.yaml 中定义的客户端名称
  -qa, --list-all                  列出所有支持的供应商（内置 + openai 兼容）
  -p, --print-format <FORMAT>      输出格式：--list-all/--webp/--webm/-q 支持 json
  -e, --execute                    用自然语言执行命令
  -c, --code                       仅输出代码
  -f, --file <FILE>                包含文件、目录或 URL
  -S, --no-stream                  关闭流式输出
      --dry-run                    只显示消息而不发送
      --info                       显示信息
      --sync-all                   从 models.dev 同步全部供应商/模型到 models-override.yaml
  -q, --find-models <KEYWORD>      搜索模型；"provider:" 限定单个供应商；多线程并行探测
      --add <MODEL>                添加/更新模型: [provider:]model | provider:*（全量导入）
  -webp, --webp <KEYWORD>         在 models.dev 搜索供应商（名称/模型数/接口/文档）
  -webm, --webm <KEYWORD>         在 models.dev 搜索模型（上下文/价格/能力）
  -webm-cn, --webm-cn <KEYWORD>   在 datalearner.com 搜索模型信息（不区分大小写）
      --out <TOOL>                 将 config.yaml 已配置模型导出到 litellm | opencode | codex 配置文件
      --list-models                列出所有可用聊天模型
      --list-roles                 列出所有角色
      --list-sessions              列出所有会话
      --list-agents                列出所有 agent
      --list-rags                  列出所有 RAG
      --list-macros                列出所有宏
  -h, --help                       显示帮助
  -V, --version                    显示版本

配置语言: 在 config.yaml 中设置 `language: cn` 可让所有提示显示中文；
未设置或其他值显示英文。
"#
    )
}
