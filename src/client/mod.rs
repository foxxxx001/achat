mod access_token;
mod common;
mod message;
#[macro_use]
mod macros;
mod model;
mod stream;

pub use crate::function::ToolCall;
pub use common::*;
pub use message::*;
pub use model::*;
pub use stream::*;

register_client!(
    (openai, "openai", OpenAIConfig, OpenAIClient),
    (
        openai_compatible,
        "openai-compatible",
        OpenAICompatibleConfig,
        OpenAICompatibleClient
    ),
    (
        openai_responses,
        "openai-responses",
        OpenAIResponsesConfig,
        OpenAIResponsesClient
    ),
    (gemini, "gemini", GeminiConfig, GeminiClient),
    (claude, "claude", ClaudeConfig, ClaudeClient),
    (cohere, "cohere", CohereConfig, CohereClient),
    (
        azure_openai,
        "azure-openai",
        AzureOpenAIConfig,
        AzureOpenAIClient
    ),
    (vertexai, "vertexai", VertexAIConfig, VertexAIClient),
    (bedrock, "bedrock", BedrockConfig, BedrockClient),
);

pub const OPENAI_COMPATIBLE_PROVIDERS: [(&str, &str); 64] = [
    ("ai21", "https://api.ai21.com/studio/v1"),
    (
        "cloudflare",
        "https://api.cloudflare.com/client/v4/accounts/{ACCOUNT_ID}/ai/v1",
    ),
    ("deepinfra", "https://api.deepinfra.com/v1/openai"),
    ("deepseek", "https://api.deepseek.com"),
    ("ernie", "https://qianfan.baidubce.com/v2"),
    ("github", "https://models.inference.ai.azure.com"),
    ("groq", "https://api.groq.com/openai/v1"),
    ("hunyuan", "https://api.hunyuan.cloud.tencent.com/v1"),
    ("minimax", "https://api.minimax.chat/v1"),
    ("mistral", "https://api.mistral.ai/v1"),
    ("moonshot", "https://api.moonshot.cn/v1"),
    ("openrouter", "https://openrouter.ai/api/v1"),
    ("perplexity", "https://api.perplexity.ai"),
    (
        "qianwen",
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
    ),
    ("xai", "https://api.x.ai/v1"),
    ("zhipuai", "https://open.bigmodel.cn/api/paas/v4"),
    // added from models.dev catalog
    ("agnes", "https://apihub.agnes-ai.com/v1"),
    ("amd", "https://developer.amd.com.cn/radeon/api/v1"),
    ("modelscope", "https://api-inference.modelscope.cn/v1"),
    ("siliconflow", "https://api.siliconflow.com/v1"),
    ("siliconflow-cn", "https://api.siliconflow.cn/v1"),
    ("volcengine", "https://ark.cn-beijing.volces.com/api/v3"),
    ("nvidia", "https://integrate.api.nvidia.com/v1"),
    ("fireworks-ai", "https://api.fireworks.ai/inference/v1/"),
    ("huggingface", "https://router.huggingface.co/v1"),
    ("baseten", "https://inference.baseten.co/v1"),
    ("chutes", "https://llm.chutes.ai/v1"),
    ("novita-ai", "https://api.novita.ai/openai"),
    ("nebius", "https://api.tokenfactory.nebius.com/v1"),
    ("friendli", "https://api.friendli.ai/serverless/v1"),
    ("gmicloud", "https://api.gmi-serving.com/v1"),
    ("digitalocean", "https://inference.do-ai.run/v1"),
    ("hetzner", "https://inference.hetzner.com/api/v1"),
    ("scaleway", "https://api.scaleway.ai/v1"),
    ("ovhcloud", "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1"),
    ("crusoe", "https://api.inference.crusoecloud.com/v1"),
    ("vultr", "https://api.vultrinference.com/v1"),
    ("io-net", "https://api.intelligence.io.solutions/api/v1"),
    ("302ai", "https://api.302.ai/v1"),
    ("poe", "https://api.poe.com/v1"),
    ("llama", "https://api.llama.com/compat/v1/"),
    ("lmstudio", "http://127.0.0.1:1234/v1"),
    ("ollama-cloud", "https://ollama.com/v1"),
    ("alibaba", "https://dashscope-intl.aliyuncs.com/compatible-mode/v1"),
    ("alibaba-cn", "https://dashscope.aliyuncs.com/compatible-mode/v1"),
    ("stepfun", "https://api.stepfun.com/v1"),
    ("stepfun-ai", "https://api.stepfun.ai/v1"),
    ("zai", "https://api.z.ai/api/paas/v4"),
    ("zhipuai-coding-plan", "https://open.bigmodel.cn/api/coding/paas/v4"),
    ("iflowcn", "https://apis.iflow.cn/v1"),
    ("moonshotai", "https://api.moonshot.ai/v1"),
    ("moonshotai-cn", "https://api.moonshot.cn/v1"),
    ("qiniu-ai", "https://api.qnaigc.com/v1"),
    ("xiaomi", "https://api.xiaomimimo.com/v1"),
    ("longcat", "https://api.longcat.chat/openai"),
    ("tencent-tokenhub", "https://tokenhub.tencentmaas.com/v1"),
    ("upstage", "https://api.upstage.ai/v1/solar"),
    ("morph", "https://api.morphllm.com/v1"),
    ("arcee", "https://api.arcee.ai/api/v1"),
    ("inference", "https://inference.net/v1"),
    ("hyper", "https://hyper.charm.land/v1"),
    ("jiekou", "https://api.jiekou.ai/openai"),
    // RAG-dedicated
    ("jina", "https://api.jina.ai/v1"),
    ("voyageai", "https://api.voyageai.com/v1"),
];
/// Extract (name, api_base, api_key, model names) from all openai-compatible clients.
pub fn compatible_client_info(config: &crate::config::Config) -> Vec<(String, String, Option<String>)> {
    use crate::client::OpenAICompatibleClient;
    config
        .clients
        .iter()
        .filter_map(|c| match c {
            ClientConfig::OpenAICompatibleConfig(cfg) => Some((
                cfg.name.clone().unwrap_or_else(|| OpenAICompatibleClient::NAME.to_string()),
                cfg.api_base.clone().unwrap_or_default(),
                cfg.api_key.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// Mutate the models of the openai-compatible client named `name`.
pub fn upsert_compatible_model(config: &mut crate::config::Config, name: &str, model: &str) -> bool {
    use crate::client::OpenAICompatibleClient;
    for c in config.clients.iter_mut() {
        if let ClientConfig::OpenAICompatibleConfig(cfg) = c {
            let this_name = cfg.name.clone().unwrap_or_else(|| OpenAICompatibleClient::NAME.to_string());
            if this_name == name {
                if !cfg.models.iter().any(|m| m.name == model) {
                    cfg.models.push(crate::client::ModelData::new(model));
                }
                return true;
            }
        }
    }
    false
}


