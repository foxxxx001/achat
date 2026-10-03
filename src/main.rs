mod cli;
mod client;
mod config;
mod i18n;
mod providers;
mod providers_cmd;
mod out_cmd;
mod function;
mod rag;
mod render;
mod repl;
mod serve;
#[macro_use]
mod utils;

#[macro_use]
extern crate log;

use crate::cli::Cli;
use crate::client::{
    call_chat_completions, call_chat_completions_streaming, list_client_types, list_models,
    ModelType,
};
use crate::config::{
    ensure_parent_exists, list_agents, load_env_file, macro_execute, Config, GlobalConfig, Input,
    WorkingMode, CODE_ROLE, EXPLAIN_SHELL_ROLE, SHELL_ROLE, TEMP_SESSION_NAME,
};
use crate::render::render_error;
use crate::repl::Repl;
use crate::utils::*;

use anyhow::{bail, Result};
use clap::Parser;
use inquire::Text;
use parking_lot::RwLock;
use simplelog::{format_description, ConfigBuilder, LevelFilter, SimpleLogger, WriteLogger};
use std::{env, process, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    load_env_file()?;
    let mut argv: Vec<String> = env::args().collect();
    crate::cli::normalize_args(&mut argv);
    // Chinese help override: clap's derived help is static English.
    let wants_help = argv.iter().any(|a| a == "--help" || a == "-h");
    if wants_help && crate::i18n::is_cn() {
        print!("{}", crate::i18n::chinese_help());
        return Ok(());
    }
    let cli = Cli::parse_from(argv);
    crate::cli::set_output_format(cli.print_format.clone());
    let text = cli.text()?;
    let working_mode = if cli.name.is_some() || cli.init || cli.list_name {
        WorkingMode::Cmd
    } else if cli.serve.is_some() {
        WorkingMode::Serve
    } else if text.is_none() && cli.file.is_empty() {
        WorkingMode::Repl
    } else {
        WorkingMode::Cmd
    };
    if cli.list_all {
        use crate::i18n::is_cn;
        use crate::providers::PROVIDERS;
        if cli.print_format.as_deref() == Some("json") {
            let items: Vec<serde_json::Value> = PROVIDERS
                .iter()
                .map(|(n, b, o)| serde_json::json!({"name": n, "api_base": b, "open": *o == "y"}))
                .collect();
            println!("{}", serde_json::to_string_pretty(&items)?);
            return Ok(());
        }
        if is_cn() {
            println!("共 {} 个供应商\n", PROVIDERS.len());
            println!("{:<22} {:<55} open", "name", "api_base");
        } else {
            println!("Total: {} providers\n", PROVIDERS.len());
            println!("{:<22} {:<55} open", "name", "api_base");
        }
        for (n, b, o) in PROVIDERS.iter() {
            println!("{:<22} {:<55} {o}", n, b);
        }
        return Ok(());
    }

    let info_flag = cli.info
        || cli.list_models
        || cli.list_roles
        || cli.list_agents
        || cli.list_rags
        || cli.list_macros
        || cli.list_sessions
        || cli.name.is_some()
        || cli.init
        || cli.list_name;
    setup_logger(working_mode.is_serve())?;
    if cli.update_providers {
        return crate::providers_cmd::update_builtin_providers(create_abort_signal()).await;
    }

    if let Some(kw) = &cli.webm_cn {
        return crate::providers_cmd::webm_cn(kw).await;
    }

    if let Some(kw) = &cli.webp {
        return crate::providers_cmd::web_providers(&kw, create_abort_signal()).await;
    }

    if let Some(kw) = &cli.webm {
        return crate::providers_cmd::web_models(&kw, create_abort_signal()).await;
    }

    let config = Arc::new(RwLock::new(Config::init(working_mode, info_flag).await?));
    if let Err(err) = run(config, cli, text).await {
        render_error(err);
        std::process::exit(1);
    }
    Ok(())
}

async fn run(config: GlobalConfig, cli: Cli, text: Option<String>) -> Result<()> {
    let abort_signal = create_abort_signal();


    if let Some(tool) = &cli.out {
        return crate::out_cmd::export(&config.read(), tool).await;
    }

    if cli.sync_all {
        return Config::sync_all_models(abort_signal).await;
    }

    if cli.list_name {
        use crate::client::list_client_names;
        let names = list_client_names(&config.read());
        if names.is_empty() {
            if crate::i18n::is_cn() {
                println!("config.yaml 中没有客户端，运行 'achat --init' 添加一个。");
            } else {
                println!("No clients defined in config.yaml. Run 'achat --init' to add one.");
            }
        } else {
            for name in names {
                println!("{name}");
            }
        }
        return Ok(());
    }

    if cli.init {
        return crate::config::init_config(&Config::config_file()).await;
    }

    if let Some(name) = &cli.name {
        return fetch_client_models(&config, name).await;
    }

    if let Some(kw) = &cli.find_models {
        let results = crate::providers_cmd::find_models(&config, kw).await?;
        crate::providers_cmd::print_find_results(&results);
        return Ok(());
    }

    if let Some(spec) = &cli.add_models {
        return crate::providers_cmd::add_models(&config, spec).await;
    }

    if cli.list_models {
        for model in list_models(&config.read(), ModelType::Chat) {
            println!("{}", model.id());
        }
        return Ok(());
    }
    if cli.list_roles {
        let roles = Config::list_roles(true).join("\n");
        println!("{roles}");
        return Ok(());
    }
    if cli.list_agents {
        let agents = list_agents().join("\n");
        println!("{agents}");
        return Ok(());
    }
    if cli.list_rags {
        let rags = Config::list_rags().join("\n");
        println!("{rags}");
        return Ok(());
    }
    if cli.list_macros {
        let macros = Config::list_macros().join("\n");
        println!("{macros}");
        return Ok(());
    }

    if cli.dry_run {
        config.write().dry_run = true;
    }

    if let Some(agent) = &cli.agent {
        let session = cli.session.as_ref().map(|v| match v {
            Some(v) => v.as_str(),
            None => TEMP_SESSION_NAME,
        });
        if !cli.agent_variable.is_empty() {
            config.write().agent_variables = Some(
                cli.agent_variable
                    .chunks(2)
                    .map(|v| (v[0].to_string(), v[1].to_string()))
                    .collect(),
            );
        }

        let ret = Config::use_agent(&config, agent, session, abort_signal.clone()).await;
        config.write().agent_variables = None;
        ret?;
    } else {
        if let Some(prompt) = &cli.prompt {
            config.write().use_prompt(prompt)?;
        } else if let Some(name) = &cli.role {
            config.write().use_role(name)?;
        } else if cli.execute {
            config.write().use_role(SHELL_ROLE)?;
        } else if cli.code {
            config.write().use_role(CODE_ROLE)?;
        }
        if let Some(session) = &cli.session {
            config
                .write()
                .use_session(session.as_ref().map(|v| v.as_str()))?;
        }
        if let Some(rag) = &cli.rag {
            Config::use_rag(&config, Some(rag), abort_signal.clone()).await?;
        }
    }
    if cli.list_sessions {
        let sessions = config.read().list_sessions().join("\n");
        println!("{sessions}");
        return Ok(());
    }
    if let Some(model_id) = &cli.model {
        config.write().set_model(model_id)?;
    }
    if cli.no_stream {
        config.write().stream = false;
    }
    if cli.empty_session {
        config.write().empty_session()?;
    }
    if cli.save_session {
        config.write().set_save_session_this_time()?;
    }
    if cli.info {
        let info = config.read().info()?;
        println!("{info}");
        return Ok(());
    }
    if let Some(addr) = cli.serve {
        return serve::run(config, addr).await;
    }
    let is_repl = config.read().working_mode.is_repl();
    if cli.rebuild_rag {
        Config::rebuild_rag(&config, abort_signal.clone()).await?;
        if is_repl {
            return Ok(());
        }
    }
    if let Some(name) = &cli.macro_name {
        macro_execute(&config, name, text.as_deref(), abort_signal.clone()).await?;
        return Ok(());
    }
    if cli.execute && !is_repl {
        let input = create_input(&config, text, &cli.file, abort_signal.clone()).await?;
        shell_execute(&config, &SHELL, input, abort_signal.clone()).await?;
        return Ok(());
    }
    config.write().apply_prelude()?;
    match is_repl {
        false => {
            let mut input = create_input(&config, text, &cli.file, abort_signal.clone()).await?;
            input.use_embeddings(abort_signal.clone()).await?;
            start_directive(&config, input, cli.code, abort_signal).await
        }
        true => {
            if !*IS_STDOUT_TERMINAL {
                bail!("No TTY for REPL")
            }
            start_interactive(&config).await
        }
    }
}

#[async_recursion::async_recursion]
async fn start_directive(
    config: &GlobalConfig,
    input: Input,
    code_mode: bool,
    abort_signal: AbortSignal,
) -> Result<()> {
    let client = input.create_client()?;
    let extract_code = !*IS_STDOUT_TERMINAL && code_mode;
    config.write().before_chat_completion(&input)?;
    let (output, tool_results) = if !input.stream() || extract_code {
        call_chat_completions(
            &input,
            true,
            extract_code,
            client.as_ref(),
            abort_signal.clone(),
        )
        .await?
    } else {
        call_chat_completions_streaming(&input, client.as_ref(), abort_signal.clone()).await?
    };
    config
        .write()
        .after_chat_completion(&input, &output, &tool_results)?;

    if !tool_results.is_empty() {
        start_directive(
            config,
            input.merge_tool_results(output, tool_results),
            code_mode,
            abort_signal,
        )
        .await?;
    }

    config.write().exit_session()?;
    Ok(())
}

async fn start_interactive(config: &GlobalConfig) -> Result<()> {
    let mut repl: Repl = Repl::init(config)?;
    repl.run().await
}

#[async_recursion::async_recursion]
async fn shell_execute(
    config: &GlobalConfig,
    shell: &Shell,
    mut input: Input,
    abort_signal: AbortSignal,
) -> Result<()> {
    let client = input.create_client()?;
    config.write().before_chat_completion(&input)?;
    let (eval_str, _) =
        call_chat_completions(&input, false, true, client.as_ref(), abort_signal.clone()).await?;

    config
        .write()
        .after_chat_completion(&input, &eval_str, &[])?;
    if eval_str.is_empty() {
        bail!("No command generated");
    }
    if config.read().dry_run {
        config.read().print_markdown(&eval_str)?;
        return Ok(());
    }
    if *IS_STDOUT_TERMINAL {
        let options = ["execute", "revise", "describe", "copy", "quit"];
        let command = color_text(eval_str.trim(), nu_ansi_term::Color::Rgb(255, 165, 0));
        let first_letter_color = nu_ansi_term::Color::Cyan;
        let prompt_text = options
            .iter()
            .map(|v| format!("{}{}", color_text(&v[0..1], first_letter_color), &v[1..]))
            .collect::<Vec<String>>()
            .join(&dimmed_text(" | "));
        loop {
            println!("{command}");
            let answer_char =
                read_single_key(&['e', 'r', 'd', 'c', 'q'], 'e', &format!("{prompt_text}: "))?;

            match answer_char {
                'e' => {
                    debug!("{} {:?}", shell.cmd, &[&shell.arg, &eval_str]);
                    let code = run_command(&shell.cmd, &[&shell.arg, &eval_str], None)?;
                    if code == 0 && config.read().save_shell_history {
                        let _ = append_to_shell_history(&shell.name, &eval_str, code);
                    }
                    process::exit(code);
                }
                'r' => {
                    let revision = Text::new("Enter your revision:").prompt()?;
                    let text = format!("{}\n{revision}", input.text());
                    input.set_text(text);
                    return shell_execute(config, shell, input, abort_signal.clone()).await;
                }
                'd' => {
                    let role = config.read().retrieve_role(EXPLAIN_SHELL_ROLE)?;
                    let input = Input::from_str(config, &eval_str, Some(role));
                    if input.stream() {
                        call_chat_completions_streaming(
                            &input,
                            client.as_ref(),
                            abort_signal.clone(),
                        )
                        .await?;
                    } else {
                        call_chat_completions(
                            &input,
                            true,
                            false,
                            client.as_ref(),
                            abort_signal.clone(),
                        )
                        .await?;
                    }
                    println!();
                    continue;
                }
                'c' => {
                    set_text(&eval_str)?;
                    println!("{}", dimmed_text("✓ Copied the command."));
                }
                _ => {}
            }
            break;
        }
    } else {
        println!("{eval_str}");
    }
    Ok(())
}

async fn create_input(
    config: &GlobalConfig,
    text: Option<String>,
    file: &[String],
    abort_signal: AbortSignal,
) -> Result<Input> {
    let input = if file.is_empty() {
        Input::from_str(config, &text.unwrap_or_default(), None)
    } else {
        Input::from_files_with_spinner(
            config,
            &text.unwrap_or_default(),
            file.to_vec(),
            None,
            abort_signal,
        )
        .await?
    };
    if input.is_empty() {
        bail!("No input");
    }
    Ok(input)
}

async fn fetch_client_models(config: &GlobalConfig, name: &str) -> Result<()> {
    use crate::client::ClientConfig;
    use serde_json::Value;

    let (client_name, api_base, api_key, extra) = {
        let config = config.read();
        let client_config = config
            .clients
            .iter()
            .find(|v| client_config_name(v) == name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Client '{}' not found in config.yaml, available clients: {}",
                    name,
                    config
                        .clients
                        .iter()
                        .map(client_config_name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        match client_config {
            ClientConfig::OpenAIConfig(c) => (
                "openai",
                c.api_base
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".into()),
                c.api_key.clone(),
                c.extra.clone(),
            ),
            ClientConfig::OpenAICompatibleConfig(c) => (
                "openai-compatible",
                c.api_base.clone().unwrap_or_default(),
                c.api_key.clone(),
                c.extra.clone(),
            ),
            ClientConfig::GeminiConfig(c) => (
                "gemini",
                c.api_base.clone()
                    .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta/openai".into()),
                c.api_key.clone(),
                c.extra.clone(),
            ),
            ClientConfig::ClaudeConfig(c) => (
                "claude",
                c.api_base.clone()
                    .unwrap_or_else(|| "https://api.anthropic.com/v1".into()),
                c.api_key.clone(),
                c.extra.clone(),
            ),
            ClientConfig::CohereConfig(c) => (
                "cohere",
                c.api_base.clone()
                    .unwrap_or_else(|| "https://api.cohere.com/compatibility/v1".into()),
                c.api_key.clone(),
                c.extra.clone(),
            ),
            ClientConfig::AzureOpenAIConfig(c) => (
                "azure-openai",
                c.api_base.clone().unwrap_or_default(),
                c.api_key.clone(),
                c.extra.clone(),
            ),
            _ => bail!("Client '{}' does not support the models api", name),
        }
    };
    let _ = client_name;
    let api_base = api_base.trim_end_matches('/');
    let url = format!("{api_base}/models");
    debug!("fetch models from {url}");

    let mut builder = reqwest::Client::builder();
    let timeout = extra
        .as_ref()
        .and_then(|v| v.connect_timeout)
        .unwrap_or(10);
    if let Some(proxy) = extra.as_ref().and_then(|v| v.proxy.as_deref()) {
        builder = set_proxy(builder, proxy)?;
    }
    let http_client = builder
        .connect_timeout(std::time::Duration::from_secs(timeout))
        .build()?;

    let mut req = http_client.get(&url);
    if let Some(api_key) = &api_key {
        req = req.bearer_auth(api_key);
    }
    if let Some(user_agent) = config.read().user_agent.as_ref() {
        req = req.header("User-Agent", user_agent);
    }

    let res = req.send().await?;
    let status = res.status();
    let data: Value = res.json().await?;
    if !status.is_success() {
        if let Some(err) = data["error"]["message"].as_str() {
            bail!("{} (status: {})", err, status.as_u16());
        }
        bail!("Invalid response data: {data} (status: {})", status.as_u16());
    }
    println!("{}", serde_json::to_string_pretty(&data)?);
    Ok(())
}

fn client_config_name(config: &crate::client::ClientConfig) -> &str {
    use crate::client::ClientConfig;
    match config {
        ClientConfig::OpenAIConfig(c) => c.name.as_deref().unwrap_or("openai"),
        ClientConfig::OpenAICompatibleConfig(c) => c.name.as_deref().unwrap_or("openai-compatible"),
        ClientConfig::OpenAIResponsesConfig(c) => c.name.as_deref().unwrap_or("openai-responses"),
        ClientConfig::GeminiConfig(c) => c.name.as_deref().unwrap_or("gemini"),
        ClientConfig::ClaudeConfig(c) => c.name.as_deref().unwrap_or("claude"),
        ClientConfig::CohereConfig(c) => c.name.as_deref().unwrap_or("cohere"),
        ClientConfig::AzureOpenAIConfig(c) => c.name.as_deref().unwrap_or("azure-openai"),
        ClientConfig::VertexAIConfig(c) => c.name.as_deref().unwrap_or("vertexai"),
        ClientConfig::BedrockConfig(c) => c.name.as_deref().unwrap_or("bedrock"),
        ClientConfig::Unknown => "",
    }
}

fn setup_logger(is_serve: bool) -> Result<()> {
    let (log_level, log_path) = Config::log_config(is_serve)?;
    if log_level == LevelFilter::Off {
        return Ok(());
    }
    let crate_name = env!("CARGO_CRATE_NAME");
    let log_filter = match std::env::var(get_env_name("log_filter")) {
        Ok(v) => v,
        Err(_) => match is_serve {
            true => format!("{crate_name}::serve"),
            false => crate_name.into(),
        },
    };
    let config = ConfigBuilder::new()
        .add_filter_allow(log_filter)
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
        ))
        .set_thread_level(LevelFilter::Off)
        .build();
    match log_path {
        None => {
            SimpleLogger::init(log_level, config)?;
        }
        Some(log_path) => {
            ensure_parent_exists(&log_path)?;
            let log_file = std::fs::File::create(log_path)?;
            WriteLogger::init(log_level, config, log_file)?;
        }
    }
    Ok(())
}
