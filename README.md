# AChat: All-in-one LLM CLI Tool

**English** | [简体中文](./readme_zh.md)

AChat (fork of AIChat v0.30.0) is an all-in-one LLM CLI tool featuring Shell Assistant, CMD & REPL Mode, RAG, AI Tools & Agents, an OpenAI-compatible serve mode (including the `/v1/responses` API), and more.

## Features

- **All-in-one LLM CLI**: CMD & interactive REPL modes, sessions, roles, agents, RAG, macros — everything in one binary (Windows & Linux).
- **200+ built-in providers**: OpenAI, Claude, Gemini, DeepSeek, Zhipu, Qwen, ModelScope and any OpenAI-compatible endpoint; `-qa`/`--list-all` lists them with api_base and keyless-availability flags.
- **OpenAI `/v1/responses` API support**: the original fork motivation, alongside classic chat completions.
- **`--init` incremental wizard**: interactively add clients one at a time, config.yaml written after every step; `-qn`/`--list-name` lists configured clients.
- **Model discovery tools**:
  - `-q <KEYWORD>`: multi-threaded (CPU-core count) parallel probe of configured + keyless-open providers
  - `--add [provider:]model | provider:*`: verify upstream then write into config.yaml
  - `-webp` / `-webm`: search the models.dev catalog (providers / models with limits, pricing, capability flags)
  - `-webm-cn`: search the datalearner.com model directory (Chinese model database)
  - `--sync-all`: pull the whole models.dev catalog into models-override.yaml
- **Weighted round-robin `-m model`**: bare model names load-balance across every client serving that model; `weight:` controls the ratio.
- **Export to other tools (`--out`)**: merge configured models into litellm (model_list), opencode.json (provider.models) or codex config.toml (model_providers) — idempotent, with .bak backups.
- **Bilingual UI**: `language: cn` in config.yaml (or a Chinese locale) switches prompts and `--help` to Chinese; otherwise English.
- **JSON output**: `-p json` for `--list-all`, `-q`, `-webp`, `-webm`, `-webm-cn`.
- **Serve mode**: OpenAI-compatible API server including the `/v1/responses` endpoint, plus a Web UI.

## Configuration

On first launch AChat asks you to pick a provider and writes `~/.config/achat/config.yaml`. To redo or extend this at any time:

```sh
achat --init
```

`--init` behaves exactly like the first-run wizard and **saves config.yaml after every provider you complete**, so it is safe to interrupt with Ctrl-C. When a config already exists, it appends new clients to it.

To refresh the whole provider/model catalog from [models.dev](https://models.dev) (writes `models-override.yaml` in the config dir, with each provider's API base URL as a comment):

```sh
achat --sync-all
```

To see which clients are configured:

```sh
$ achat --list-name
mock-provider
test2
```

To see every provider AChat supports:

```sh
$ achat --list-all            # table view
$ achat --list-all -p json    # JSON array output
Total: 63 providers

name                   api_base                                               open
ai21                   https://api.ai21.com/studio/v1                         y
deepinfra              https://api.deepinfra.com/v1/openai                    y
deepseek               https://api.deepseek.com                               n
...

`open=y` means the provider's `/v1/models` endpoint works without an API key.
```

### Finding & adding models

```sh
achat -q glm                        # search open + configured providers for "glm"
achat -p json -q glm                # same search, JSON array output
achat -q openrouter:               # list only openrouter's models
achat --add openrouter:zai-org/glm-5.3   # add one model to a specific provider
achat --add glm-5.3                      # try every configured provider
achat --add 'openrouter:*'              # import ALL of a provider's models
```

### datalearner.com model search

```sh
achat -webm-cn glm                  # search datalearner.com models (case-insensitive)
achat -p json --webm-cn glm         # JSON output
```

### Export models to other tools (`--out`)

```sh
achat --out litellm                 # merge config.yaml models into litellm config.yaml (model_list)
achat --out opencode                # merge into opencode.json provider.models
achat --out codex                   # add [model_providers.*] sections to ~/.codex/config.toml
```
`-qa` is a shorthand alias of `--list-all`.

### models.dev queries

```sh
achat --webp glm                    # search models.dev providers (id/name/api/doc)
achat --webp openai:deepseek        # restrict to OpenAI-compatible providers
achat -p json --webp deepseek       # JSON output
achat --webm glm-4.6                # search all models.dev models (limits, pricing, flags)
achat -p json --webm glm-4.6        # JSON output
```

### Weighted round-robin (`-m model_name`)

When `-m` receives a bare model name, achat load-balances across every configured
client that offers that model. Give a model a `weight` to control the distribution:

```yaml
clients:
  - type: openai-compatible
    name: provider-a
    models:
      - name: glm-5.3
        weight: 3    # picked 3x more often than weight:1 peers
```

## CLI Options

| Option | Description |
| --- | --- |
| `-m, --model <MODEL>` | Select a LLM model |
| `--prompt <PROMPT>` | Use the system prompt |
| `-r, --role <ROLE>` | Select a role |
| `-s, --session [<SESSION>]` | Start or join a session |
| `--empty-session` | Ensure the session is empty |
| `--save-session` | Ensure the new conversation is saved to the session |
| `-a, --agent <AGENT>` | Start a agent |
| `--agent-variable <NAME> <VALUE>` | Set agent variables |
| `--rag <RAG>` | Start a RAG |
| `--rebuild-rag` | Rebuild the RAG to sync document changes |
| `--macro <MACRO>` | Execute a macro |
| `--serve [<ADDRESS>]` | Serve the LLM API and WebAPP |
| `-n, --name <NAME>` | Select a client by name and fetch its /v1/models via HTTP GET |
| `--init` | Initialize/extend the config file interactively (like first-run wizard); saves config.yaml after each entry |
| `--list-name` | List all client names defined in config.yaml |
| `--list-all` | List all supported providers (built-in clients + openai-compatible providers) |
| `-e, --execute` | Execute commands in natural language |
| `-c, --code` | Output code only |
| `-f, --file <FILE>` | Include files, directories, or URLs |
| `-S, --no-stream` | Turn off stream mode |
| `--dry-run` | Display the message without sending it |
| `--info` | Display information |
| `-q <KEYWORD>` | Search models in parallel (one worker thread per CPU core); `provider:` lists only that provider's models |
| `-p <FORMAT>` | Output format for `--list-all` and `-q`: `json` prints a JSON array |
| `--add <[provider:]model\|provider:*>` | Verify upstream and add/update model(s) in config.yaml; `provider:*` imports all |
| `--sync-all` | Sync all providers/models from the models.dev catalog into models.yaml |
| `--list-models` | List all available chat models |
| `--list-roles` | List all roles |
| `--list-sessions` | List all sessions |
| `--list-agents` | List all agents |
| `--list-rags` | List all RAGs |
| `--list-macros` | List all macros |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

### UI language (config.yaml)

```yaml
# UI language: `cn` shows all prompts in Chinese; unset or any other value shows English
language: cn
```

## Environment Variables

AChat reads its configuration from (in order of precedence):

- `ACHAT_CONFIG_FILE` — path to the config file
- `ACHAT_LANG` — override UI language (`zh`/`en`); the config.yaml `language: cn` setting takes precedence
- `ACHAT_CONFIG_DIR` — config directory (default `~/.config/achat`)
- `ACHAT_LANG` — force UI language: `zh` (Chinese) or `en` (English). Without it, Windows machines in the China region automatically get Chinese messages.

## Examples

```sh
achat                           # REPL mode
achat hello                     # one-shot chat
achat -m claude:claude-3-5-sonnet "hi"
achat -e list the 10 largest files here   # shell assistant
achat --serve 127.0.0.1:8008    # OpenAI-compatible API server
achat -n my-provider            # fetch a client's /v1/models
achat --init                    # add another provider to config.yaml
achat --list-name               # show configured client names
achat --list-all                # show all supported providers
```
