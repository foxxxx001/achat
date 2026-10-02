# AChat：一体化 LLM 命令行工具

AChat（基于 AIChat v0.30.0 的分支）是一体化 LLM 命令行工具，具备 Shell 助手、CMD 与 REPL 模式、RAG、AI 工具与代理，以及兼容 OpenAI 的服务模式（含 `/v1/responses` API）等功能。

## 特性

- **内置 70+ 供应商**（OpenAI、Claude、Gemini、Ollama、Groq、Azure-OpenAI、Deepseek、通义千问、Moonshot、ModelScope（魔搭）、Agnes AI、AMD、SiliconFlow（硅基流动）、火山引擎、Nvidia、Fireworks、HuggingFace 等，以及任意 OpenAI 兼容接口）
- CMD 与交互式 REPL 模式、会话、角色、代理、RAG、宏
- Shell 助手（`-e`）：用自然语言生成并执行 shell 命令
- 内置 API 服务器（`--serve`）：提供 `/v1/chat/completions` **和** OpenAI `/v1/responses` 接口
- 首次运行向导与增量式 `--init` 配置构建

## 配置

首次启动时 AChat 会引导你选择供应商并生成 `~/.config/achat/config.yaml`。随时可以重新运行或扩展配置：

```sh
achat --init
```

`--init` 与首次运行向导完全一致，并且**每完成一个供应商就立即保存 config.yaml**，中途 Ctrl-C 退出也不会丢失已输入的内容。若配置文件已存在，新的客户端会追加到现有配置中。

从 [models.dev](https://models.dev) 刷新整个供应商/模型目录（写入配置目录下的 `models-override.yaml`，每个供应商的接口地址以注释形式标出）：

```sh
achat --sync-all
```

查看已配置的客户端：

```sh
$ achat --list-name
mock-provider
test2
```

查看 AChat 支持的全部供应商：

```sh
$ achat --list-all
Total: 70 providers
openai
claude
...
```

## 命令行参数

| 参数 | 说明 |
| --- | --- |
| `-m, --model <MODEL>` | 选择 LLM 模型 |
| `--prompt <PROMPT>` | 使用系统提示词 |
| `-r, --role <ROLE>` | 选择角色 |
| `-s, --session [<SESSION>]` | 开始或加入会话 |
| `--empty-session` | 确保会话为空 |
| `--save-session` | 确保新对话保存到会话 |
| `-a, --agent <AGENT>` | 启动代理 |
| `--agent-variable <NAME> <VALUE>` | 设置代理变量 |
| `--rag <RAG>` | 启动 RAG |
| `--rebuild-rag` | 重建 RAG 以同步文档变更 |
| `--macro <MACRO>` | 执行宏 |
| `--serve [<ADDRESS>]` | 启动 LLM API 与 WebAPP 服务 |
| `-n, --name <NAME>` | 按名称选择客户端，通过 HTTP GET 获取其 /v1/models |
| `--init` | 交互式初始化/扩展配置文件（同首次运行向导）；每录入一项即保存 config.yaml |
| `--list-name` | 列出 config.yaml 中定义的所有客户端名称 |
| `--list-all` | 列出所有支持的供应商（内置客户端 + OpenAI 兼容供应商） |
| `-e, --execute` | 用自然语言执行命令 |
| `-c, --code` | 仅输出代码 |
| `-f, --file <FILE>` | 附带文件、目录或 URL |
| `-S, --no-stream` | 关闭流式输出 |
| `--dry-run` | 仅显示消息而不发送 |
| `--info` | 显示信息 |
| `--sync-models` | 同步模型更新 |
| `--sync-all` | 从 models.dev 目录同步全部供应商/模型到 models.yaml |
| `--list-models` | 列出所有可用的对话模型 |
| `--list-roles` | 列出所有角色 |
| `--list-sessions` | 列出所有会话 |
| `--list-agents` | 列出所有代理 |
| `--list-rags` | 列出所有 RAG |
| `--list-macros` | 列出所有宏 |
| `-h, --help` | 打印帮助 |
| `-V, --version` | 打印版本号 |

## 环境变量

AChat 按以下优先级读取配置：

- `ACHAT_CONFIG_FILE` — 配置文件路径
- `ACHAT_CONFIG_DIR` — 配置目录（默认 `~/.config/achat`）
- `ACHAT_LANG` — 强制界面语言：`zh`（中文）或 `en`（英文）。未设置时，位于中国区的 Windows 系统自动显示中文提示。

## 示例

```sh
achat                           # REPL 模式
achat hello                     # 单次对话
achat -m claude:claude-3-5-sonnet "hi"
achat -e 列出当前目录最大的 10 个文件   # Shell 助手
achat --serve 127.0.0.1:8008    # 启动 OpenAI 兼容 API 服务器
achat -n my-provider            # 获取客户端的 /v1/models
achat --init                    # 向 config.yaml 添加新的供应商
achat --list-name               # 显示已配置的客户端名称
achat --list-all                # 显示所有支持的供应商
```
