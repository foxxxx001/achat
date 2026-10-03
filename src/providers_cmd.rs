//! `--find-models` and `--add-models` implementations.

use crate::client::{compatible_client_info, upsert_compatible_model};
use crate::config::GlobalConfig;
use crate::providers::PROVIDERS;
use anyhow::{bail, Context, Result};
use serde_json::Value;

fn extract_model_names(body: &str) -> Vec<String> {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    // jq path: .data[].name (fall back to .data[].id, a common variant)
    let mut names = vec![];
    if let Some(items) = v.get("data").and_then(|d| d.as_array()) {
        for item in items {
            if let Some(n) = item.get("name").and_then(|x| x.as_str()) {
                names.push(n.to_string());
            } else if let Some(n) = item.get("id").and_then(|x| x.as_str()) {
                names.push(n.to_string());
            }
        }
    }
    names
}

async fn fetch_models(api_base: &str, api_key: Option<&str>) -> Result<Vec<String>> {
    let url = format!("{}/models", api_base.trim_end_matches('/'));
    let mut req = reqwest::Client::new().get(&url);
    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }
    let resp = req
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .with_context(|| format!("Failed to request '{url}'"))?;
    if !resp.status().is_success() {
        bail!("HTTP {}", resp.status());
    }
    let body = resp.text().await?;
    Ok(extract_model_names(&body))
}

/// --find-models: search providers from config.yaml that carry an api_key plus
/// every provider with open=y; print `provider_name model_name` for matches.
pub async fn find_models(config: &GlobalConfig, keyword: &str) -> Result<Vec<(String, String)>> {
    // "provider:" (trailing colon) => only list models of that provider.
    // Otherwise search the keyword across configured (api_key) + open providers.
    let (only_provider, kw) = if keyword.ends_with(':') {
        (Some(keyword.trim_end_matches(':').to_string()), None)
    } else {
        (None, Some(keyword.to_lowercase()))
    };

    let mut targets: Vec<(String, String, Option<String>)> = compatible_client_info(&config.read())
        .into_iter()
        .filter(|(_, _, key)| key.as_deref().map(|k| !k.is_empty()).unwrap_or(false))
        .collect();
    for (n, b, o) in PROVIDERS.iter() {
        if *o == "y" && !targets.iter().any(|(name, _, _)| name == n) {
            targets.push((n.to_string(), b.to_string(), None));
        }
    }
    if let Some(p) = &only_provider {
        targets.retain(|(name, _, _)| name == p);
        if targets.is_empty() {
            // unknown provider: fall back to its registry entry (no key) if open
            if let Some((b, o)) = crate::providers::provider_info(p) {
                if o == "y" {
                    targets.push((p.clone(), b.to_string(), None));
                }
            }
        }
        if targets.is_empty() {
            bail!("Provider '{p}' not found in config.yaml or open provider list");
        }
    }

    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    debug!("find-models: probing with {threads} threads");

    let (tx, rx) = std::sync::mpsc::channel::<Vec<(String, String)>>();
    let chunk_size = (targets.len() + threads - 1) / threads.max(1);
    let mut handles = vec![];
    for chunk in targets.chunks(chunk_size.max(1)).map(|c| c.to_vec()) {
        let tx = tx.clone();
        let kw = kw.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            let mut local = vec![];
            for (name, base, key) in &chunk {
                let models = rt.block_on(fetch_models(base, key.as_deref()));
                if let Ok(models) = models {
                    for m in models {
                        if let Some(kw) = &kw {
                            if !m.to_lowercase().contains(kw) {
                                continue;
                            }
                        }
                        local.push((name.clone(), m));
                    }
                }
            }
            let _ = tx.send(local);
        }));
    }
    drop(tx);
    for h in handles {
        let _ = h.await;
    }
    let mut results = vec![];
    while let Ok(mut part) = rx.try_recv() {
        results.append(&mut part);
    }
    results.sort();
    Ok(results)
}

/// Print find-models results (text or JSON depending on -p).
pub fn print_find_results(results: &[(String, String)]) {
    if results.is_empty() {
        println!("No models matched.");
        return;
    }
    if crate::cli::output_format() == Some("json") {
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|(p, m)| serde_json::json!({"provider": p, "model": m}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&items).unwrap_or_default());
    } else {
        for (p, m) in results {
            println!("{p}:{m}");
        }
    }
}

/// --add-models: `[provider:]model_name`. Verify the model exists upstream via
/// /v1/models, then add/update it under the matching client in config.yaml.
pub async fn add_models(config: &GlobalConfig, spec: &str) -> Result<()> {
    // `provider:*` => fetch all models of that provider and import them all.
    if let Some((p, "*")) = spec.split_once(':') {
        if !p.is_empty() {
            return import_all_models(config, p).await;
        }
    }
    let (provider, model) = match spec.split_once(':') {
        Some((p, m)) if !p.is_empty() && !m.is_empty() => (Some(p.to_string()), m.to_string()),
        _ => (None, spec.to_string()),
    };
    if model.is_empty() {
        bail!("Invalid model spec '{spec}'");
    }

    let mut candidates: Vec<(String, String, Option<String>)> =
        compatible_client_info(&config.read());
    if let Some(p) = &provider {
        candidates.retain(|(name, _, _)| name == p);
    }
    if candidates.is_empty() {
        if let Some(p) = &provider {
            bail!("Provider '{p}' not found in config.yaml");
        }
        bail!("No providers found in config.yaml. Run 'achat --init' first.");
    }

    for (name, base, key) in &candidates {
        let models = match fetch_models(base, key.as_deref()).await {
            Ok(m) => m,
            Err(err) => {
                eprintln!("! {name}: {err}");
                continue;
            }
        };
        if models.iter().any(|m| *m == model) {
            upsert_client_model_in_file(name, &model)?;
            println!("✓ {name} {model} added/updated in config.yaml");
            return Ok(());
        }
    }
    bail!("Model '{model}' not found on any candidate provider");
}

/// Update config.yaml on disk: add model to the named client (no-op if present).
fn upsert_client_model_in_file(client_name: &str, model: &str) -> Result<()> {
    use crate::config::Config;
    let path = Config::config_file();
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse config.yaml")?;

    let clients = yaml
        .get_mut("clients")
        .and_then(|v| v.as_sequence_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid config: 'clients' is not a list"))?;

    for client in clients.iter_mut() {
        let is_target = client.get("name").and_then(|v| v.as_str()) == Some(client_name);
        if !is_target {
            continue;
        }
        let models = client
            .as_mapping_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid client entry"))?
            .entry(serde_yaml::Value::from("models"))
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        let seq = models
            .as_sequence_mut()
            .ok_or_else(|| anyhow::anyhow!("'models' is not a list"))?;
        let exists = seq
            .iter()
            .any(|m| m.get("name").and_then(|v| v.as_str()) == Some(model));
        if !exists {
            let mut entry = serde_yaml::Mapping::new();
            entry.insert(
                serde_yaml::Value::from("name"),
                serde_yaml::Value::from(model),
            );
            seq.push(serde_yaml::Value::Mapping(entry));
        }
        std::fs::write(&path, serde_yaml::to_string(&yaml)?)
            .with_context(|| format!("Failed to write '{}'", path.display()))?;
        return Ok(());
    }
    bail!("Client '{client_name}' not found in config.yaml");
}

/// `-add provider:*`: fetch the provider's /v1/models and import every model
/// into that provider's entry in config.yaml.
async fn import_all_models(config: &GlobalConfig, provider: &str) -> Result<()> {
    let candidates: Vec<(String, String, Option<String>)> = compatible_client_info(&config.read());
    let (name, base, key) = candidates
        .into_iter()
        .find(|(n, _, _)| n == provider)
        .ok_or_else(|| anyhow::anyhow!("Provider '{provider}' not found in config.yaml"))?;

    let models = fetch_models(&base, key.as_deref()).await?;
    if models.is_empty() {
        bail!("Provider '{provider}' returned no models");
    }
    let path = crate::config::Config::config_file();
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse config.yaml")?;
    let clients = yaml
        .get_mut("clients")
        .and_then(|v| v.as_sequence_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid config: 'clients' is not a list"))?;

    for client in clients.iter_mut() {
        if client.get("name").and_then(|v| v.as_str()) != Some(provider) {
            continue;
        }
        let mapping = client
            .as_mapping_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid client entry"))?;
        let models_entry = mapping
            .entry(serde_yaml::Value::from("models"))
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        let seq = models_entry
            .as_sequence_mut()
            .ok_or_else(|| anyhow::anyhow!("'models' is not a list"))?;
        let mut added = 0;
        for m in &models {
            let exists = seq
                .iter()
                .any(|e| e.get("name").and_then(|v| v.as_str()) == Some(m.as_str()));
            if !exists {
                let mut entry = serde_yaml::Mapping::new();
                entry.insert(
                    serde_yaml::Value::from("name"),
                    serde_yaml::Value::from(m.as_str()),
                );
                seq.push(serde_yaml::Value::Mapping(entry));
                added += 1;
            }
        }
        std::fs::write(&path, serde_yaml::to_string(&yaml)?)
            .with_context(|| format!("Failed to write '{}'", path.display()))?;
        println!(
            "✓ {provider}: {} models total, {added} newly added to config.yaml",
            models.len()
        );
        return Ok(());
    }
    bail!("Client '{provider}' not found in config.yaml");
}

// ---------------------------------------------------------------------------
// models.dev queries: `--webp` / `--webm` and the dev helper `--update-providers`
// ---------------------------------------------------------------------------

const CATALOG_URL: &str = "https://models.dev/api.json";

/// Number of worker threads = detected CPU core count.
fn worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Fetch the whole models.dev catalog (one request, a few MB).
async fn fetch_catalog(abort_signal: &crate::utils::AbortSignal) -> Result<serde_json::Map<String, Value>> {
    let _ = abort_signal;
    let resp = reqwest::Client::new()
        .get(CATALOG_URL)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .with_context(|| format!("Failed to request '{CATALOG_URL}'"))?;
    if !resp.status().is_success() {
        bail!("HTTP {} from '{CATALOG_URL}'", resp.status());
    }
    let body = resp.text().await?;
    let v: Value = serde_json::from_str(&body).with_context(|| "Failed to parse models.dev catalog")?;
    v.as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("models.dev catalog is not an object"))
}

/// --webp <keyword>: case-insensitive search of models.dev providers. Each result
/// shows the provider name, model count, API base and documentation URL. Prefix
/// the keyword with `openai:` to restrict to OpenAI-compatible providers.
pub async fn web_providers(keyword: &str, abort_signal: crate::utils::AbortSignal) -> Result<()> {
    let catalog = fetch_catalog(&abort_signal).await?;
    let (openai_only, kw) = match keyword.strip_prefix("openai:") {
        Some(k) => (true, k.to_lowercase()),
        None => (false, keyword.to_lowercase()),
    };

    let threads = worker_threads();
    let entries: Vec<(String, Value)> = catalog.into_iter().collect();
    let chunk_size = (entries.len() + threads - 1) / threads.max(1);
    let (tx, rx) = std::sync::mpsc::channel::<Vec<Value>>();
    let mut handles = vec![];
    for chunk in entries.chunks(chunk_size.max(1)).map(|c| c.to_vec()) {
        let tx = tx.clone();
        let kw = kw.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            let mut local = vec![];
            for (id, e) in chunk {
                let name = e.get("name").and_then(|v| v.as_str()).unwrap_or(&id);
                let api = e.get("api").and_then(|v| v.as_str()).unwrap_or("");
                let doc = e.get("doc").and_then(|v| v.as_str()).unwrap_or("");
                let npm = e.get("npm").and_then(|v| v.as_str()).unwrap_or("");
                let openai_compat = npm.contains("openai");
                if openai_only && !openai_compat {
                    continue;
                }
                let hay = format!("{id} {name} {api} {doc}").to_lowercase();
                if !hay.contains(&kw) {
                    continue;
                }
                let count = e
                    .get("models")
                    .and_then(|v| v.as_object())
                    .map(|m| m.len())
                    .unwrap_or(0);
                local.push(serde_json::json!({
                    "id": id, "name": name, "models": count,
                    "api_base": api, "doc": doc, "openai_compatible": openai_compat,
                }));
            }
            let _ = tx.send(local);
        }));
    }
    drop(tx);
    for h in handles {
        let _ = h.await;
    }
    let mut rows = vec![];
    while let Ok(mut part) = rx.try_recv() {
        rows.append(&mut part);
    }
    rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));

    if rows.is_empty() {
        println!("No providers matched '{keyword}' on models.dev.");
        return Ok(());
    }
    if crate::cli::output_format() == Some("json") {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    println!(
        "{:<24} {:<26} {:>6}  {:<46} {}",
        "id", "name", "models", "api_base", "doc"
    );
    for r in &rows {
        println!(
            "{:<24} {:<26} {:>6}  {:<46} {}",
            truncate(r["id"].as_str().unwrap_or(""), 22),
            truncate(r["name"].as_str().unwrap_or(""), 24),
            r["models"].as_u64().unwrap_or(0),
            truncate(r["api_base"].as_str().unwrap_or(""), 44),
            truncate(r["doc"].as_str().unwrap_or(""), 40)
        );
    }
    println!("\n{} providers matched.", rows.len());
    Ok(())
}

/// --webm <keyword>: case-insensitive search of every model in the models.dev
/// catalog; shows provider, model id/name, context & output limits, pricing and
/// capability flags. Rows are collected by worker threads (= CPU cores).
pub async fn web_models(keyword: &str, abort_signal: crate::utils::AbortSignal) -> Result<()> {
    let catalog = fetch_catalog(&abort_signal).await?;
    let kw = keyword.to_lowercase();

    let threads = worker_threads();
    debug!("webm: searching with {threads} threads");
    let entries: Vec<(String, Value)> = catalog.into_iter().collect();
    let chunk_size = (entries.len() + threads - 1) / threads.max(1);
    let (tx, rx) = std::sync::mpsc::channel::<Vec<Value>>();
    let mut handles = vec![];
    for chunk in entries.chunks(chunk_size.max(1)).map(|c| c.to_vec()) {
        let tx = tx.clone();
        let kw = kw.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            let mut local = vec![];
            for (pid, e) in chunk {
                let models = match e.get("models").and_then(|v| v.as_object()) {
                    Some(m) => m,
                    None => continue,
                };
                for (mid, m) in models {
                    let name = m.get("name").and_then(|v| v.as_str()).unwrap_or(mid);
                    let hay = format!("{pid} {mid} {name}").to_lowercase();
                    if !hay.contains(&kw) {
                        continue;
                    }
                    let limit = m.get("limit");
                    let cost = m.get("cost");
                    local.push(serde_json::json!({
                        "provider": pid, "id": mid, "name": name,
                        "context": limit.and_then(|l| l.get("context")).and_then(|v| v.as_u64()),
                        "output": limit.and_then(|l| l.get("output")).and_then(|v| v.as_u64()),
                        "input_price": cost.and_then(|c| c.get("input")).and_then(|v| v.as_f64()),
                        "output_price": cost.and_then(|c| c.get("output")).and_then(|v| v.as_f64()),
                        "vision": m.get("attachment").and_then(|v| v.as_bool()).unwrap_or(false),
                        "reasoning": m.get("reasoning").and_then(|v| v.as_bool()).unwrap_or(false),
                        "tool_call": m.get("tool_call").and_then(|v| v.as_bool()).unwrap_or(false),
                    }));
                }
            }
            let _ = tx.send(local);
        }));
    }
    drop(tx);
    for h in handles {
        let _ = h.await;
    }
    let mut rows = vec![];
    while let Ok(mut part) = rx.try_recv() {
        rows.append(&mut part);
    }
    rows.sort_by(|a, b| {
        (a["provider"].as_str(), a["id"].as_str()).cmp(&(b["provider"].as_str(), b["id"].as_str()))
    });

    if rows.is_empty() {
        println!("No models matched '{keyword}' on models.dev.");
        return Ok(());
    }
    if crate::cli::output_format() == Some("json") {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    println!(
        "{:<20} {:<42} {:>9} {:>9} {:>9} {:>9}  {}",
        "provider", "model", "context", "output", "in$/M", "out$/M", "flags"
    );
    for r in &rows {
        let mut flags = String::new();
        if r["vision"].as_bool().unwrap_or(false) {
            flags.push('V');
        }
        if r["reasoning"].as_bool().unwrap_or(false) {
            flags.push('R');
        }
        if r["tool_call"].as_bool().unwrap_or(false) {
            flags.push('T');
        }
        let label = format!(
            "{} [{}]",
            r["id"].as_str().unwrap_or(""),
            r["name"].as_str().unwrap_or("")
        );
        println!(
            "{:<20} {:<42} {:>9} {:>9} {:>9} {:>9}  {}",
            truncate(r["provider"].as_str().unwrap_or(""), 18),
            truncate(&label, 40),
            fmt_num(r["context"].as_u64()),
            fmt_num(r["output"].as_u64()),
            fmt_price(r["input_price"].as_f64()),
            fmt_price(r["output_price"].as_f64()),
            flags
        );
    }
    println!(
        "\n{} models matched. Flags: V=vision R=reasoning T=tool-call",
        rows.len()
    );
    Ok(())
}

/// `--update-providers` (dev helper): fetch models.dev and append every
/// OpenAI-compatible provider missing from the built-in `src/providers.rs`.
/// Run before compiling to keep the registry current.
pub async fn update_builtin_providers(abort_signal: crate::utils::AbortSignal) -> Result<()> {
    let catalog = fetch_catalog(&abort_signal).await?;

    let known: std::collections::HashSet<String> =
        PROVIDERS.iter().map(|(n, _, _)| n.to_string()).collect();
    let old_len = PROVIDERS.len();

    let threads = worker_threads();
    debug!("update-providers: scanning with {threads} threads");
    let entries: Vec<(String, Value)> = catalog.into_iter().collect();
    let chunk_size = (entries.len() + threads - 1) / threads.max(1);
    let (tx, rx) = std::sync::mpsc::channel::<Vec<(String, String, String)>>();
    let known = std::sync::Arc::new(known);
    let mut handles = vec![];
    for chunk in entries.chunks(chunk_size.max(1)).map(|c| c.to_vec()) {
        let tx = tx.clone();
        let known = known.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            let mut local = vec![];
            for (id, e) in chunk {
                if known.contains(&id) {
                    continue;
                }
                let api = e.get("api").and_then(|v| v.as_str()).unwrap_or("");
                let npm = e.get("npm").and_then(|v| v.as_str()).unwrap_or("");
                // Only OpenAI-compatible providers belong in the registry.
                if api.is_empty() || !npm.contains("openai") {
                    continue;
                }
                let name = e
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&id)
                    .to_string();
                local.push((id, name, api.to_string()));
            }
            let _ = tx.send(local);
        }));
    }
    drop(tx);
    for h in handles {
        let _ = h.await;
    }
    let mut candidates = vec![];
    while let Ok(mut part) = rx.try_recv() {
        candidates.append(&mut part);
    }
    candidates.sort();

    if candidates.is_empty() {
        println!(
            "✓ Built-in provider list is up to date ({} providers).",
            old_len
        );
        return Ok(());
    }

    let path = std::path::Path::new("src/providers.rs");
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut new_content = content.clone();
    let mut added_names = vec![];
    for (id, name, api) in &candidates {
        if new_content.contains(&format!("\"{id}\"")) {
            continue;
        }
        let pos = new_content
            .find("\n];")
            .ok_or_else(|| anyhow::anyhow!("Could not locate the PROVIDERS array end"))?;
        new_content.insert_str(pos, &format!("\n    (\"{id}\", \"{api}\", \"n\"), // {name}"));
        added_names.push((id.clone(), name.clone(), api.clone()));
    }

    let new_len = old_len + added_names.len();
    if new_len != old_len {
        new_content = new_content.replace(
            &format!("[(&str, &str, &str); {old_len}]"),
            &format!("[(&str, &str, &str); {new_len}]"),
        );
        std::fs::write(path, &new_content)
            .with_context(|| format!("Failed to write '{}'", path.display()))?;
        println!(
            "✓ Added {} providers to {} ({} → {}):",
            added_names.len(),
            path.display(),
            old_len,
            new_len
        );
        for (id, name, api) in &added_names {
            println!("  + {id:<24} {name:<28} {api}");
        }
        println!("\nRe-run `cargo fmt` / rebuild to apply.");
    } else {
        println!("✓ Built-in provider list is up to date ({} providers).", old_len);
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

fn fmt_num(v: Option<u64>) -> String {
    match v {
        Some(n) if n >= 1_000_000 => format!("{}M", n / 1_000_000),
        Some(n) if n >= 1_000 => format!("{}k", n / 1_000),
        Some(n) => n.to_string(),
        None => "-".into(),
    }
}

fn fmt_price(v: Option<f64>) -> String {
    match v {
        Some(p) => format!("{p:.2}"),
        None => "-".into(),
    }
}

// ---------------------------------------------------------------------------
// --webm-cn: search datalearner.com (AI大模型列表)
// ---------------------------------------------------------------------------

/// --webm-cn <keyword>: query datalearner.com's pretrained-model list
/// (case-insensitive keyword). The site is a Next.js app; its SSR search
/// endpoint `?q=<keyword>` embeds a full JSON model list (initialBootstrap).
pub async fn webm_cn(keyword: &str) -> Result<()> {
    let url = format!(
        "https://www.datalearner.com/ai-models/pretrained-models?q={}",
        urlencoding::encode(keyword)
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(45))
        .send()
        .await
        .with_context(|| format!("Failed to request '{url}'"))?;
    if !resp.status().is_success() {
        bail!("HTTP {} from '{url}'", resp.status());
    }
    let html = resp.text().await?;

    // Pull the Next.js flight payloads and decode escapes.
    let mut full = String::new();
    let re = fancy_regex::Regex::new(r#"self\.__next_f\.push\(\[1,"(.*?)"\]\)"#)
        .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;
    for caps in re.captures_iter(&html).flatten() {
        let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let unescaped = unescape_js(raw);
        full.push_str(&unescaped);
    }

    // Locate the "models":[ ... ] array inside initialBootstrap.
    let idx = full
        .find("\"models\":[")
        .ok_or_else(|| anyhow::anyhow!("未找到模型数据（页面结构可能已变化）"))?;
    let arr_start = idx + "\"models\":".len();
    let bytes = full.as_bytes();
    if bytes.get(arr_start) != Some(&b'[') {
        bail!("模型数据格式异常");
    }
    let mut depth = 0usize;
    let mut in_str = false;
    let mut prev_escape = false;
    let mut end = arr_start;
    for (i, &b) in bytes[arr_start..].iter().enumerate() {
        let c = b as char;
        if in_str {
            if prev_escape {
                prev_escape = false;
            } else if c == '\\' {
                prev_escape = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = arr_start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end <= arr_start {
        bail!("模型数据解析失败");
    }
    let arr_text = &full[arr_start..end];
    let models: Vec<Value> = serde_json::from_str(arr_text)
        .with_context(|| "解析 datalearner 模型数据失败")?;

    // Collect results.
    let kw = keyword.to_lowercase();
    let mut rows: Vec<(String, String, String, String, String)> = vec![]; // code, name, org, type, publish
    for m in &models {
        let code = m.get("model_code").and_then(|v| v.as_str()).unwrap_or("");
        let name = m
            .get("model_abbr_name")
            .and_then(|v| v.as_str())
            .unwrap_or(code);
        let hay = format!("{code} {name}").to_lowercase();
        if !hay.contains(&kw) {
            continue;
        }
        let org = m
            .get("aiOrganization")
            .and_then(|v| v.get("org_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let typ = m
            .get("modelType")
            .and_then(|v| v.get("nameZh"))
            .and_then(|v| v.as_str())
            .or_else(|| m.get("model_TYPE_NAME").and_then(|v| v.as_str()))
            .unwrap_or("-");
        let publish = m
            .get("publish_time")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let params = m
            .get("parameterSizeDisplay")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        rows.push((
            code.to_string(),
            name.to_string(),
            format!("{org} / {typ}"),
            params.to_string(),
            publish.to_string(),
        ));
    }

    if rows.is_empty() {
        println!("datalearner.com 未找到匹配 '{keyword}' 的模型。");
        return Ok(());
    }
    if crate::cli::output_format() == Some("json") {
        let items: Vec<Value> = models
            .iter()
            .filter(|m| {
                let code = m.get("model_code").and_then(|v| v.as_str()).unwrap_or("");
                let name = m
                    .get("model_abbr_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("{code} {name}").to_lowercase().contains(&kw)
            })
            .cloned()
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }
    println!(
        "{:<32} {:<26} {:<24} {:>8}  {}",
        "model", "name", "org/type", "params", "published"
    );
    for (code, name, orgtyp, params, publish) in &rows {
        println!(
            "{:<32} {:<26} {:<24} {:>8}  {}",
            truncate(code, 30),
            truncate(name, 24),
            truncate(orgtyp, 22),
            params,
            publish
        );
    }
    println!(
        "\n{} models matched. 详情: https://www.datalearner.com/ai-models/pretrained-models/<model>",
        rows.len()
    );
    Ok(())
}

/// Decode a JS string literal body (\n, \", \\, \uXXXX).
fn unescape_js(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                    if let Some(ch) = char::from_u32(cp) {
                        out.push(ch);
                    }
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
