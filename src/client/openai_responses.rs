use super::*;
use crate::utils::strip_think_tag;

use anyhow::{bail, Context, Result};
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OpenAIResponsesConfig {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization_id: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelData>,
    pub patch: Option<RequestPatch>,
    pub extra: Option<ExtraConfig>,
}

impl OpenAIResponsesClient {
    config_get_fn!(api_key, get_api_key);
    config_get_fn!(api_base, get_api_base);

    pub const PROMPTS: [PromptAction<'static>; 1] = [("api_key", "API Key", None)];
}

impl_client_trait!(
    OpenAIResponsesClient,
    (
        prepare_chat_completions,
        openai_responses_chat_completions,
        openai_responses_chat_completions_streaming
    ),
    (noop_prepare_embeddings, noop_embeddings),
    (noop_prepare_rerank, noop_rerank),
);

fn prepare_chat_completions(
    self_: &OpenAIResponsesClient,
    data: ChatCompletionsData,
) -> Result<RequestData> {
    let api_key = self_.get_api_key()?;
    let api_base = self_
        .get_api_base()
        .unwrap_or_else(|_| API_BASE.to_string());

    prepare_responses_request(
        &api_base,
        Some(&api_key),
        data,
        &self_.model,
        self_.config.organization_id.as_deref(),
    )
}

/// Client wrapper for the OpenAI `/v1/responses` API.
///
/// Request:  POST {api_base}/responses with a `CreateResponse` body:
///   { model, input: [{role, content}] | "text", instructions, stream, temperature, top_p,
///     max_output_tokens, tools: [{type: "function", name, description, parameters}] }
///
/// Non-stream response: a `Response` object; output text is extracted from
///   output[] items of type "message" -> content[] of type "output_text".
///
/// Streaming response: an SSE event stream with events such as
///   `response.created`, `response.output_text.delta` (delta text),
///   `response.function_call_arguments.delta` / `.done` (tool calls),
///   `response.completed` (final response object).
pub const API_PATH: &str = "/responses";

fn build_input(messages: Vec<Message>) -> (Option<String>, Vec<Value>) {
    // Split system instructions from the rest of the conversation.
    let mut instructions = String::new();
    let mut input = vec![];
    for message in messages {
        let Message { role, content } = message;
        let text = match content {
            MessageContent::ToolCalls(MessageContentToolCalls {
                tool_results,
                text,
                ..
            }) => {
                // Emit prior tool calls and outputs as input items.
                for tool_result in tool_results {
                    input.push(json!({
                        "type": "function_call",
                        "call_id": tool_result.call.id,
                        "name": tool_result.call.name,
                        "arguments": tool_result.call.arguments.to_string(),
                    }));
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": tool_result.call.id,
                        "output": tool_result.output.to_string(),
                    }));
                }
                text
            }
            MessageContent::Text(text) => text,
            MessageContent::Array(_) => String::new(),
        };
        if text.is_empty() {
            continue;
        }
        match role {
            MessageRole::System => {
                if !instructions.is_empty() {
                    instructions.push('\n');
                }
                instructions.push_str(&strip_think_tag(&text));
            }
            MessageRole::Assistant => {
                input.push(json!({
                    "role": "assistant",
                    "type": "message",
                    "content": strip_think_tag(&text),
                }));
            }
            _ => {
                input.push(json!({
                    "role": "user",
                    "type": "message",
                    "content": text,
                }));
            }
        }
    }
    let instructions = if instructions.is_empty() {
        None
    } else {
        Some(instructions)
    };
    (instructions, input)
}

fn build_tools(functions: Option<Vec<crate::function::FunctionDeclaration>>) -> Option<Vec<Value>> {
    functions.map(|functions| {
        functions
            .iter()
            .map(|v| {
                // FunctionDeclaration serializes to {"type":"function","function":{...}}
                // (Chat Completions format); flatten it to the Responses format.
                let mut value = serde_json::to_value(v).unwrap_or_else(|_| json!({}));
                let function = value
                    .as_object_mut()
                    .and_then(|obj| obj.remove("function"))
                    .unwrap_or_else(|| json!({}));
                let mut tool = json!({"type": "function"});
                if let Some(obj) = function.as_object() {
                    for (k, val) in obj {
                        tool[k.as_str()] = val.clone();
                    }
                }
                tool
            })
            .collect()
    })
}

pub fn openai_responses_build_body(data: ChatCompletionsData, model: &Model) -> Value {
    let ChatCompletionsData {
        messages,
        temperature,
        top_p,
        functions,
        stream,
    } = data;

    let (instructions, input) = build_input(messages);

    let mut body = json!({
        "model": model.real_name(),
        "input": input,
        "stream": stream,
    });
    if let Some(instructions) = instructions {
        body["instructions"] = instructions.into();
    }
    if let Some(v) = temperature {
        body["temperature"] = v.into();
    }
    if let Some(v) = top_p {
        body["top_p"] = v.into();
    }
    if let Some(v) = model.max_tokens_param() {
        body["max_output_tokens"] = v.into();
    }
    if let Some(tools) = build_tools(functions) {
        if !tools.is_empty() {
            body["tools"] = tools.into();
        }
    }
    body
}

/// Extract the output text and tool calls from a `Response` object.
pub fn openai_responses_extract_output(data: &Value) -> Result<ChatCompletionsOutput> {
    let mut text = String::new();
    let mut tool_calls = vec![];

    if let Some(output) = data["output"].as_array() {
        for item in output {
            match item["type"].as_str() {
                Some("message") => {
                    if let Some(content) = item["content"].as_array() {
                        for part in content {
                            if part["type"].as_str() == Some("output_text") {
                                if let Some(t) = part["text"].as_str() {
                                    text.push_str(t);
                                }
                            }
                        }
                    }
                }
                Some("function_call") => {
                    if let (Some(name), Some(arguments)) =
                        (item["name"].as_str(), item["arguments"].as_str())
                    {
                        let arguments: Value = arguments.parse().with_context(|| {
                            format!("Tool call '{name}' have non-JSON arguments '{arguments}'")
                        })?;
                        tool_calls.push(ToolCall::new(
                            name.to_string(),
                            arguments,
                            item["call_id"].as_str().map(|v| v.to_string()),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    if text.is_empty() && tool_calls.is_empty() {
        bail!("Invalid response data: {data}");
    }
    let output = ChatCompletionsOutput {
        text,
        tool_calls,
        id: data["id"].as_str().map(|v| v.to_string()),
        input_tokens: data["usage"]["input_tokens"].as_u64(),
        output_tokens: data["usage"]["output_tokens"].as_u64(),
    };
    Ok(output)
}

pub async fn openai_responses_chat_completions(
    builder: RequestBuilder,
    _model: &Model,
) -> Result<ChatCompletionsOutput> {
    let res = builder.send().await?;
    let status = res.status();
    let data: Value = res.json().await?;
    if !status.is_success() {
        catch_error(&data, status.as_u16())?;
    }
    debug!("responses-non-stream-data: {data}");
    openai_responses_extract_output(&data)
}

pub async fn openai_responses_chat_completions_streaming(
    builder: RequestBuilder,
    handler: &mut SseHandler,
    _model: &Model,
) -> Result<()> {
    let mut function_name = String::new();
    let mut function_arguments = String::new();
    let mut function_id = String::new();

    let handle = |message: SseMmessage| -> Result<bool> {
        if message.data == "[DONE]" {
            return Ok(true);
        }
        let data: Value = serde_json::from_str(&message.data)?;
        debug!("responses-stream-data: {data}");
        match data["type"].as_str() {
            Some("response.output_text.delta") => {
                if let Some(text) = data["delta"].as_str() {
                    handler.text(text)?;
                }
            }
            Some("response.output_item.added") => {
                // A new function call item started.
                if data["item"]["type"].as_str() == Some("function_call") {
                    function_name = data["item"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    function_id = data["item"]["call_id"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    function_arguments.clear();
                }
            }
            Some("response.function_call_arguments.delta") => {
                if let Some(delta) = data["delta"].as_str() {
                    function_arguments.push_str(delta);
                }
            }
            Some("response.function_call_arguments.done") => {
                let mut arguments = data["arguments"].as_str().unwrap_or_default().to_string();
                if arguments.is_empty() {
                    arguments = function_arguments.clone();
                }
                if arguments.is_empty() {
                    arguments = "{}".into();
                }
                let arguments: Value = arguments.parse().with_context(|| {
                    format!(
                        "Tool call '{function_name}' have non-JSON arguments '{arguments}'"
                    )
                })?;
                handler.tool_call(ToolCall::new(
                    function_name.clone(),
                    arguments,
                    if function_id.is_empty() {
                        None
                    } else {
                        Some(function_id.clone())
                    },
                ))?;
                function_name.clear();
                function_arguments.clear();
                function_id.clear();
            }
            Some("response.completed") | Some("response.incomplete") => {
                return Ok(true);
            }
            Some("response.failed") => {
                let err = data["response"]["error"]["message"]
                    .as_str()
                    .unwrap_or("response failed");
                bail!("{err}");
            }
            _ => {}
        }
        Ok(false)
    };

    sse_stream(builder, handle).await
}

pub fn prepare_responses_request(
    api_base: &str,
    api_key: Option<&str>,
    data: ChatCompletionsData,
    model: &Model,
    organization_id: Option<&str>,
) -> Result<RequestData> {
    let api_base = api_base.trim_end_matches('/');
    let url = format!("{api_base}{API_PATH}");

    let body = openai_responses_build_body(data, model);

    let mut request_data = RequestData::new(url, body);

    if let Some(api_key) = api_key {
        request_data.bearer_auth(api_key);
    }
    if let Some(organization_id) = organization_id {
        request_data.header("OpenAI-Organization", organization_id);
    }
    Ok(request_data)
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ResponsesUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}
