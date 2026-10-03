[English](./README.md) | **简体中文**
# AChat：一体化 LLM 命令行工具

[English](./README.md) | **简体中文**

AChat（基于 AIChat v0.30.0 的分支）是一体化 LLM 命令行工具，具备 Shell 助手、CMD 与 REPL 模式、RAG、AI 工具与代理，以及兼容 OpenAI 的服务模式（含 `/v1/responses` API）等功能。

## 特性

- **一体化 LLM 命令行工具**：CMD 与交互式 REPL 模式、会话、角色、代理、RAG、宏，全部集成在一个二进制里（支持 Windows 与 Linux）。
- **内置 200+ 供应商**：OpenAI、Claude、Gemini、DeepSeek、智谱、通义千问、ModelScope（魔搭）及任意 OpenAI 兼容接口；`-qa`/`--list-all` 列出全部供应商的接口地址与免密可用标志。
- **OpenAI `/v1/responses` API 支持**：本分支的最初动机，同时兼容经典 chat completions。
- **`--init` 增量式向导**：逐个交互式添加客户端，每完成一个即写盘 config.yaml；`-qn`/`--list-name` 列出已配置客户端。
- **模型发现工具**：
  - `-q <关键词>`：按 CPU 核心数多线程并行探测已配置 + 免密开放的供应商
  - `--add [provider:]model | provider:*`：先向上游验证再写入 config.yaml
  - `-webp` / `-webm`：搜索 models.dev 目录（供应商 / 模型的上下文、价格、能力标志）
  - `-webm-cn`：搜索 datalearner.com 模型库（中文大模型数据库）
  - `--sync-all`：将整个 models.dev 目录拉取到 models-override.yaml
- **权重轮询 `-m model`**：纯模型名会在所有提供该模型的客户端间负载均衡；`weight:` 控制比例。
- **导出到其他工具（`--out`）**：将已配置模型合并进 litellm（model_list）、opencode.json（provider.models）或 codex config.toml（model_providers）——幂等操作，自动写 .bak 备份。
- **中英双语界面**：config.yaml 中 `language: cn`（或中文区域）切换全部提示与 `--help` 为中文，否则英文。
- **JSON 输出**：`-p json` 适用于 `--list-all`、`-q`、`-webp`、`-webm`、`-webm-cn`。
- **服务模式**：兼容 OpenAI 的 API 服务（含 `/v1/responses` 端点）+ Web UI。

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
Total: 62 providers

name                   api_base                                               open
ai21                   https://api.ai21.com/studio/v1                         y
deepinfra              https://api.deepinfra.com/v1/openai                    y
deepseek               https://api.deepseek.com                               n
...

`open=y` 表示该供应商的 `/v1/models` 接口无需 API 密钥即可访问。
```

### 查找与添加模型

```sh
achat -q glm                        # 在开放 + 已配置供应商中搜索 "glm"
achat -p json -q glm                # 同样的搜索，JSON 数组输出
achat -q openrouter:               # 只列出 openrouter 的模型
achat --add openrouter:zai-org/glm-5.3   # 添加单个模型到指定供应商
achat --add glm-5.3                      # 轮询所有已配置供应商
achat --add 'openrouter:*'              # 导入该供应商的全部模型
```

### datalearner.com 模型搜索

```sh
achat --webm-cn glm                 # 在 datalearner.com 搜索模型（不区分大小写）
achat -p json --webm-cn glm         # JSON 输出
```

### 导出模型到其他工具（`--out`）

```sh
achat --out litellm                 # 将 config.yaml 模型合并进 litellm 配置（model_list）
achat --out opencode                # 合并进 opencode.json 的 provider.models
achat --out codex                   # 向 ~/.codex/config.toml 添加 [model_providers.*]
```
`-qa` 是 `--list-all` 的简写别名。

### models.dev 在线查询

```sh
achat --webp glm                    # 在 models.dev 搜索供应商（id/名称/接口/文档）
achat --webp openai:deepseek        # 仅限 OpenAI 兼容供应商
achat -p json --webp deepseek       # JSON 输出
achat --webm glm-4.6                # 搜索 models.dev 全部模型（上下文/价格/能力）
achat -p json --webm glm-4.6        # JSON 输出
```

### 权重轮询（`-m model_name`）

当 `-m` 只给模型名（不带 `provider:` 前缀）时，achat 会在所有提供该模型的客户端间负载均衡。
为模型设置 `weight` 可控制分配比例：

```yaml
clients:
  - type: openai-compatible
    name: provider-a
    models:
      - name: glm-5.3
        weight: 3    # 被选中的概率是 weight:1 的 3 倍
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
| `-q <关键词>` | 多线程并行搜索模型（线程数 = CPU 核心数）；`provider:` 只列出该供应商的模型 |
| `-p <格式>` | `--list-all` 与 `-q` 的输出格式：`json` 输出 JSON 数组 |
| `--add <[provider:]model\|provider:*>` | 上游验证后添加/更新模型到 config.yaml；`provider:*` 导入该供应商全部模型 |
| `--sync-all` | 从 models.dev 目录同步全部供应商/模型到 models.yaml |
| `--list-models` | 列出所有可用的对话模型 |
| `--list-roles` | 列出所有角色 |
| `--list-sessions` | 列出所有会话 |
| `--list-agents` | 列出所有代理 |
| `--list-rags` | 列出所有 RAG |
| `--list-macros` | 列出所有宏 |
| `-h, --help` | 打印帮助 |
| `-V, --version` | 打印版本号 |

### 界面语言（config.yaml）

```yaml
# 界面提示语言：设为 cn 时所有提示显示中文；未设置或其他值显示英文
language: cn
```

## 环境变量

AChat 按以下优先级读取配置：

- `ACHAT_CONFIG_FILE` — 配置文件路径
- `ACHAT_LANG` — 强制界面语言（`zh`/`en`）；config.yaml 的 `language: cn` 优先级更高
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
