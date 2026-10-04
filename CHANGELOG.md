## achat v0.8.9 — Patch Notes

1. `-provider <NAME>`: the api_key is now returned in full (no masking).

2. Removed the `-qa` alias (use `--list-all` or `-provider` with no argument).

3. Removed the `--list-models` flag; `-model` with no argument keeps the same
   output (configured chat models).

4. `-webp` with no argument lists ALL providers on models.dev; with a keyword
   it filters as before.

5. `-webm` with no argument lists ALL models on models.dev; with a keyword it
   filters as before. New options: `-o json` (JSON output) and `--free`
   (only models whose in$/M AND out$/M are both 0).

## achat v0.8.8 — Patch Notes

1. `-qn` renamed to `-provider` / `--provider` with new semantics:
   - `-provider` (no argument): identical output to `-qa`/`--list-all` — lists
     all built-in + OpenAI-compatible providers (name / api_base / open).
   - `-provider <NAME>`: finds every client in config.yaml whose name contains
     NAME (case-insensitive), calls its `/v1/models`, and prints api_base,
     api_key (masked) and the returned model ids.

2. `-q` renamed to `-model` / `--find-models`:
   - `-model` (no argument): same output as `--list-models`.
   - `-model <KEYWORD>`: same multi-threaded model search as before.

3. Both artifacts (Windows exe and Linux binary) are UPX-compressed from this
   release on.

## achat v0.8.7 — Patch Notes

1. `-webm-cn`: new shorthand alias of `--webm-cn`.

## achat v0.8.6 — Patch Notes

1. `-qn`: new shorthand alias of `--list-name`.
2. `-webp` / `-webm`: new shorthand aliases of `--webp` / `--webm`.
3. README.md: added a language switch line (`English | 简体中文`) and a
   rewritten `## Features` section summarizing achat's main capabilities;
   readme_zh.md got the matching `## 特性` and switch line.
4. `patch.md` (source root): new markdown changelog recording every version's
   patch notes under a version-number heading (history backfilled);
   `patch.txt` removed.
5. Release artifacts now carry the version suffix in every file name.

## achat v0.8.5 — Patch Notes

1. `-qa`: new shorthand alias of `--list-all` (the long option still works).

2. `--help` is now localized: with `language: cn` in config.yaml (or a Chinese
   locale), `achat --help` prints a full Chinese help text (clap's derived
   help is static English, so it is overridden at runtime).

3. `--webm-cn <KEYWORD>`: search datalearner.com AI model list
   (https://www.datalearner.com/ai-models/pretrained-models) case-insensitively.
   Shows model code, name, organization/type, parameter size and publish date;
   `-p json` outputs the raw matched model records.

4. `--version` no longer shows "forked from sigoden/aichat" (now
   "achat <version> / Made by Gary-China").

5. `--out <TOOL>`: export models configured in config.yaml to another tool:
   - `litellm`: merge into litellm config.yaml `model_list`
     (model: openai/<model>, api_base, api_key); detects LITELLM_CONFIG_PATH,
     ./litellm.config.yaml or ~/litellm/config.yaml.
   - `opencode`: merge into opencode.json `provider.<client>` entries with
     npm @ai-sdk/openai-compatible, baseURL/apiKey options and a models map.
   - `codex`: append `[model_providers.<client>]` sections (base_url, env_key)
     to ~/.codex/config.toml; API keys are referenced via ACHAT_*_API_KEY env
     vars, model defaults are written as commented suggestions.
   Existing entries are never overwritten; a .bak backup is written before
   modifying any file.

## achat v0.8.4 — Patch Notes

1. `--webp <KEYWORD>`: search providers on models.dev
   - Case-insensitive match against provider id, name, API base and doc URL.
   - Shows provider name, model count, API base and documentation URL.
   - `openai:<keyword>` restricts results to OpenAI-compatible providers.
   - Works with `-p json` for JSON output.

2. `--webm <KEYWORD>`: search models on models.dev
   - Case-insensitive match against provider, model id and display name.
   - Shows provider, model id/name, context & output limits, per-million-token
     pricing and capability flags (V=vision R=reasoning T=tool-call).
   - Works with `-p json` for JSON output.

3. `--update-providers` (dev helper, hidden): fetches models.dev and appends
   every OpenAI-compatible provider missing from the built-in
   `src/providers.rs` registry — run before compiling to refresh the list.

All three use multi-threaded processing sized to the current system core
count (std::thread::available_parallelism).

## achat v0.8.3 — Patch Notes

1. Global `language` setting in config.yaml
   - `language: cn` => all prompts/messages in Chinese; unset or any other
     value => English (previously Chinese was only auto-detected on Windows in
     the China region; that fallback still applies when `language` is unset).
   - The setting takes precedence over ACHAT_LANG and auto-detection.
   - Documented in config.example.yaml.

2. `-q` (find-models) now probes providers in parallel
   - Worker thread count = current system core count
     (std::thread::available_parallelism). Results are merged and sorted, so
     output order is deterministic.

3. Version bump to 0.8.3.

## achat v0.8.2 — Patch Notes

1. `-p json` now also applies to `-q` (find-models)
   - `achat -p json -q <keyword>` prints the search results as a JSON array of
     {"provider": ..., "model": ...} objects instead of the plain
     `provider:model` lines. `-p json --list-all` works as before.
   - Without `-p json`, `-q` output is unchanged (`provider:model` per line).

2. Version bump to 0.8.2 (third digit only, per convention).

## achat v0.8.1 — Patch Notes

1. Sensenova added to the provider registry
   - name: sensenova
   - api_base: https://token.sensenova.cn/v1
   - open: n (probed: /v1/models requires an API key, HTTP 401)
   Registry now covers 63 providers.

2. `--list-all -p json`
   - New `-p <FORMAT>` option. `-p json` prints the provider list as a JSON array
     of {"name": ..., "api_base": ..., "open": true|false} objects. Without -p
     the aligned three-column table is printed as before.

3. `-q` replaces `--find-models`
   - `-q <keyword>` searches as before (configured providers with api_key plus
     every open=y provider), but results now print in `provider:model` format.
   - `-q <provider>:` (trailing colon) lists ALL models of that provider only.
     Providers not present in config.yaml are still resolved through the
     internal registry when they are open (no key needed).

4. `--add` replaces `--add-models`
   - `--add <provider>:<model>` behaves as before (verify upstream, then
     add/update the model under that provider in config.yaml).
   - NEW: `--add <provider>:*` fetches the provider's /v1/models and imports
     every returned model into that provider's entry in config.yaml, skipping
     models that are already present. Reports total & newly added counts.
   - The flag is long-only; the previous `-add` spelling was ambiguous with
     clap short-flag clustering and has been removed.

5. Version reset to 0.8.1
   - Going forward the version increments only in the third digit (0.8.1 ->
     0.8.2 -> ...) unless a different version is explicitly requested.

## achat 0.6.0 (round 4)

### Added

- **Bilingual UI**: on Windows systems located in the China region (detected via OS locale,
  LANG/LC_ALL/LANGUAGE, TZ, or UTC+8 local offset) all messages from the new features
  (wizard/--init, --list-name, --list-all, --sync-all) are shown in Chinese; otherwise English.
  Set `ACHAT_LANG=zh` or `ACHAT_LANG=en` to force a locale.
- **Version banner**: `achat --version` now prints "Made by Gary-China, forked from sigoden/aichat".

## achat 0.6.0 (round 3)

### Added

- **`--sync-all` flag**: fetches the models.dev catalog and regenerates the local
  `models-override.yaml` (config dir) with all OpenAI-compatible providers — each provider
  annotated with a `# API Base:` comment. Verified: 218 providers written & parseable.
- **models.yaml**: every one of the 70 providers now carries a `# API Base: <url>` comment.
- **Fixed**: all 46 placeholder API URLs in `OPENAI_COMPATIBLE_PROVIDERS` replaced with real
  endpoints sourced from models.dev (agnes, amd, modelscope, siliconflow, volcengine, nvidia,
  fireworks-ai, alibaba, stepfun, moonshotai, xiaomi, longcat, ...).

# Changelog (achat)

`achat` is a fork of [aichat](https://github.com/sigoden/aichat) v0.30.0 with the following additions:

## achat 0.30.0

### Added

- **`-n/--name` flag**: Given a client `name`, read the matching `clients` entry from the
  default `config.yaml`, issue an HTTP GET to its `{api_base}/models` endpoint, and print
  the raw JSON response.
  ```sh
  achat -n my-provider
  ```

- **OpenAI Responses API (`POST /v1/responses`) support**, implemented following the
  [OpenAPI spec](https://github.com/openai/openai-openapi) (`CreateResponse` request /
  `Response` object / SSE event stream):
  - New client type `openai-responses` in config.yaml:
    ```yaml
    clients:
      - type: openai-responses
        api_key: sk-xxx
        models:
          - name: gpt-5
    ```
  - Existing `openai` / `openai-compatible` clients can opt in via `extra`:
    ```yaml
    clients:
      - type: openai-compatible
        name: my-provider
        api_base: https://example.com/v1
        api_key: sk-xxx
        extra:
          use_responses_api: true      # POST {api_base}/responses instead of /chat/completions
          responses_api_base: https://example.com/v1  # optional override
    ```
  - Built-in server (`achat --serve`) now exposes `POST /v1/responses` which accepts a
    CreateResponse body (`model`, `input` string or item list, `instructions`, `temperature`,
    `top_p`, `max_output_tokens`, `stream`, `tools`) and forwards it to the configured upstream,
    returning a `Response` object (non-stream) or the standard Responses SSE event stream
    (`response.created`, `response.output_item.added`, `response.content_part.added`,
    `response.output_text.delta`, `response.output_item.done`, `response.content_part.done`,
    `response.completed`, `response.failed`).

### Windows build

Cross-compiled Windows x64 binary is provided: `achat-x86_64-pc-windows-gnu.exe`
(build: `cargo build --release --target x86_64-pc-windows-gnu`).

## achat 0.30.0 (round 2)

### Added

- **models.yaml expanded 24 → 70 providers** (+1421 models), sourced from the models.dev
  catalog. Notable additions: **agnes** (Agnes AI), **amd** (AMD Radeon), **modelscope**
  (ModelScope), siliconflow/siliconflow-cn, volcengine-ark, nvidia, fireworks-ai,
  huggingface-router, baseten, chutes, novita, nebius, friendli, gmi-cloud, digitalocean,
  hetzner, scaleway, ovhcloud, crusoe, vultr, io-net, 302ai, poe, llama-api, lmstudio,
  ollama-cloud, alibaba/alibaba-cn (dashscope), stepfun (cn/global), zai, zhipu-coding-plan,
  iflow, moonshot (cn/global), qiniu, xiaomi, longcat, tencent-tokenhub, upstage, morph,
  arcee, inference.net, charm-hyper, jiekou — with per-model context limits, pricing,
  vision and function-calling flags.
- **`--init` flag**: interactive wizard that appends one provider per pass to config.yaml,
  saving after every entry. Creates the file when missing (same flow as first run).
- **`--list-name` flag**: prints all `name` values from the `clients` section of config.yaml.
## achat 0.72 — Release Notes (English)

### Removed

- **`--sync-models` flag removed** along with the bundled `models.yaml`. Model metadata now
  lives in `models-override.yaml` (populate it with `--sync-all`) or in your `config.yaml`.

### Added

- **Internal provider registry** (`src/providers.rs`): 62 providers with `name`, `api_base`
  and an `open` flag recording whether `/v1/models` is reachable **without** an API key
  (probed live: 12 of 62 are open — ai21, chutes, deepinfra, friendli, hyper, modelscope,
  novita-ai, nvidia, ollama-cloud, openrouter, ovhcloud, qiniu-ai).
- **`--list-all` now prints three columns**: `name`, `api_base`, `open`.
- **`--find-models <KEYWORD>`**: queries `/v1/models` on every provider in config.yaml that
  has an `api_key` plus every `open=y` provider, and prints `provider_name model_name` for
  each match of the (case-insensitive) keyword against the jq path `.data[].name`
  (with `.data[].id` fallback).
- **`--add-models <[provider:]model_name>`**: verifies the model exists upstream via
  `/v1/models`, then adds/updates it under the matching client in config.yaml.
  - `provider:model` — strict: only that provider is queried (api_key taken from config).
  - `model` — loops over every provider in config.yaml and adds the model to the first
    provider that has it.
- **`-m model_name` weighted round-robin**: when `-m` gets a bare model name (no
  `provider:` prefix), achat selects among every configured client offering that model.
  If a model entry defines `weight: N`, the selection follows weighted round-robin;
  otherwise all candidates get equal weight.

