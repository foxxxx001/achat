# AChat: All-in-one LLM CLI Tool

AChat (fork of AIChat v0.30.0) is an all-in-one LLM CLI tool featuring Shell Assistant, CMD & REPL Mode, RAG, AI Tools & Agents, an OpenAI-compatible serve mode (including the `/v1/responses` API), and more.

## Features

- **70+ providers** out of the box (OpenAI, Claude, Gemini, Ollama, Groq, Azure-OpenAI, Deepseek, Qianwen, Moonshot, ModelScope, Agnes AI, AMD, SiliconFlow, Volcengine, Nvidia, Fireworks, HuggingFace, ... any OpenAI-compatible API)
- CMD & interactive REPL modes, sessions, roles, agents, RAGs, macros
- Shell assistant (`-e`) that turns natural language into shell commands
- Built-in API server (`--serve`) exposing `/v1/chat/completions` **and** OpenAI `/v1/responses`
- First-run wizard and incremental `--init` config builder

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
$ achat --list-all
Total: 70 providers
openai
claude
...
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
| `--sync-models` | Sync models updates |
| `--sync-all` | Sync all providers/models from the models.dev catalog into models.yaml |
| `--list-models` | List all available chat models |
| `--list-roles` | List all roles |
| `--list-sessions` | List all sessions |
| `--list-agents` | List all agents |
| `--list-rags` | List all RAGs |
| `--list-macros` | List all macros |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Environment Variables

AChat reads its configuration from (in order of precedence):

- `ACHAT_CONFIG_FILE` — path to the config file
- `ACHAT_CONFIG_DIR` — config directory (default `~/.config/achat`)

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
