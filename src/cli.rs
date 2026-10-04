use anyhow::{Context, Result};
use clap::Parser;
use is_terminal::IsTerminal;
use std::io::{stdin, Read};

const VERSION_EXTRA: &str = concat!(env!("CARGO_PKG_VERSION"), "\nMade by Gary-China");

static OUTPUT_FORMAT: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

/// Normalize alias flags before clap parsing (clap shorts are single-char,
/// so `-qn` / `-webp` / `-webm` / `-model` are mapped to their long forms).
pub fn normalize_args(args: &mut Vec<String>) {
    // arg-aware: `-model` (with value) => --find-models; bare `-model` => --list-models
    let mut i = 0;
    while i < args.len() {
        let a = args[i].clone();
        match a.as_str() {
            "-webp" => args[i] = "--webp".to_string(),
            "-webm" => args[i] = "--webm".to_string(),
            "-webm-cn" => args[i] = "--webm-cn".to_string(),
            "-provider" => args[i] = "--provider".to_string(),
            "-model" => args[i] = "--find-models".to_string(),
            _ => {}
        }
        i += 1;
    }
}

/// Record the -p value at parse time for other modules to query.
pub fn output_format() -> Option<&'static str> {
    OUTPUT_FORMAT.get().and_then(|v| v.as_deref())
}

pub fn set_output_format(v: Option<String>) {
    let _ = OUTPUT_FORMAT.set(v);
}

#[derive(Parser, Debug)]
#[command(
    author,
    version = VERSION_EXTRA,
    about,
    long_about = None
)]
pub struct Cli {
    /// Select a LLM model
    #[clap(short, long)]
    pub model: Option<String>,
    /// Use the system prompt
    #[clap(long)]
    pub prompt: Option<String>,
    /// Select a role
    #[clap(short, long)]
    pub role: Option<String>,
    /// Start or join a session
    #[clap(short = 's', long)]
    pub session: Option<Option<String>>,
    /// Ensure the session is empty
    #[clap(long)]
    pub empty_session: bool,
    /// Ensure the new conversation is saved to the session
    #[clap(long)]
    pub save_session: bool,
    /// Start a agent
    #[clap(short = 'a', long)]
    pub agent: Option<String>,
    /// Set agent variables
    #[clap(long, value_names = ["NAME", "VALUE"], num_args = 2)]
    pub agent_variable: Vec<String>,
    /// Start a RAG
    #[clap(long)]
    pub rag: Option<String>,
    /// Rebuild the RAG to sync document changes
    #[clap(long)]
    pub rebuild_rag: bool,
    /// Execute a macro
    #[clap(long = "macro", value_name = "MACRO")]
    pub macro_name: Option<String>,
    /// Serve the LLM API and WebAPP
    #[clap(long, value_name = "ADDRESS")]
    pub serve: Option<Option<String>>,
    /// Select a client by name and fetch its /v1/models via HTTP GET
    #[clap(short = 'n', long, value_name = "NAME")]
    pub name: Option<String>,
    /// Initialize/extend the config file interactively (like first-run wizard)
    #[clap(long)]
    pub init: bool,
    /// List all client names defined in config.yaml (alias: -qn)
    #[clap(long = "list-name")]
    pub list_name: bool,
    /// List all supported providers (built-in clients + openai-compatible providers)
    #[clap(long = "list-all")]
    pub list_all: bool,
    /// Output format for --list-all: "json" for a JSON array
    #[clap(short = 'p', value_name = "FORMAT")]
    pub print_format: Option<String>,
    /// Execute commands in natural language
    #[clap(short = 'e', long)]
    pub execute: bool,
    /// Output code only
    #[clap(short = 'c', long)]
    pub code: bool,
    /// Include files, directories, or URLs
    #[clap(short = 'f', long, value_name = "FILE")]
    pub file: Vec<String>,
    /// Turn off stream mode
    #[clap(short = 'S', long)]
    pub no_stream: bool,
    /// Display the message without sending it
    #[clap(long)]
    pub dry_run: bool,
    /// Display information
    #[clap(long)]
    pub info: bool,
    /// Sync all providers/models from the models.dev catalog into models.yaml
    #[clap(long = "sync-all")]
    pub sync_all: bool,
    /// Search models by keyword (alias: -model <KEYWORD>); no value = --list-models
    #[clap(long = "find-models", value_name = "KEYWORD", num_args = 0..=1, default_missing_value = "")]
    pub find_models: Option<String>,
    /// List providers; with a name filter, query that provider's /v1/models (alias: -provider [NAME])
    #[clap(long = "provider", value_name = "NAME", num_args = 0..=1, default_missing_value = "")]
    pub provider: Option<String>,
    /// Add/update models in config.yaml: [provider:]model | provider:* (import all)
    #[clap(long = "add", value_name = "MODEL")]
    pub add_models: Option<String>,
    /// List models.dev providers; with keyword filter (alias: -webp [KEYWORD])
    #[clap(long = "webp", value_name = "KEYWORD", num_args = 0..=1, default_missing_value = "")]
    pub webp: Option<String>,
    /// List models.dev models; with keyword filter (alias: -webm [KEYWORD]);
    /// combine with -o json or --free (in$/M and out$/M both 0)
    #[clap(long = "webm", value_name = "KEYWORD", num_args = 0..=1, default_missing_value = "")]
    pub webm: Option<String>,
    /// Output format: "json" (for -webp / -webm etc.)
    #[clap(short = 'o', value_name = "FORMAT")]
    pub out_format: Option<String>,
    /// With --webm: only show models whose input AND output price are both 0
    #[clap(long)]
    pub free: bool,
    /// Search datalearner.com models by keyword (alias: -webm-cn)
    #[clap(long = "webm-cn", value_name = "KEYWORD")]
    pub webm_cn: Option<String>,
    /// Export config.yaml models to another tool's config: litellm | opencode | codex
    #[clap(long = "out", value_name = "TOOL")]
    pub out: Option<String>,
    /// (dev) fetch models.dev and append providers missing from the built-in list
    #[clap(long = "update-providers", hide = true)]
    pub update_providers: bool,
    /// List all roles
    #[clap(long)]
    pub list_roles: bool,
    /// List all sessions
    #[clap(long)]
    pub list_sessions: bool,
    /// List all agents
    #[clap(long)]
    pub list_agents: bool,
    /// List all RAGs
    #[clap(long)]
    pub list_rags: bool,
    /// List all macros
    #[clap(long)]
    pub list_macros: bool,
    /// Input text
    #[clap(trailing_var_arg = true)]
    text: Vec<String>,
}

impl Cli {
    pub fn text(&self) -> Result<Option<String>> {
        let mut stdin_text = String::new();
        if !stdin().is_terminal() {
            let _ = stdin()
                .read_to_string(&mut stdin_text)
                .context("Invalid stdin pipe")?;
        };
        match self.text.is_empty() {
            true => {
                if stdin_text.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(stdin_text))
                }
            }
            false => {
                if self.macro_name.is_some() {
                    let text = self
                        .text
                        .iter()
                        .map(|v| shell_words::quote(v))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if stdin_text.is_empty() {
                        Ok(Some(text))
                    } else {
                        Ok(Some(format!("{text} -- {stdin_text}")))
                    }
                } else {
                    let text = self.text.join(" ");
                    if stdin_text.is_empty() {
                        Ok(Some(text))
                    } else {
                        Ok(Some(format!("{text}\n{stdin_text}")))
                    }
                }
            }
        }
    }
}
