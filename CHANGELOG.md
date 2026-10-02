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
