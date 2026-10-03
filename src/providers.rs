//! Internal provider registry: name, api_base, and whether /v1/models is
//! accessible without an API key (`open` = "y"/"n", probed on 2026-10-02).

/// (name, api_base, open) — open=y: /v1/models works without api_key.
pub const PROVIDERS: [(&str, &str, &str); 203] = [
    ("ai21", "https://api.ai21.com/studio/v1", "y"),
    ("deepinfra", "https://api.deepinfra.com/v1/openai", "y"),
    ("deepseek", "https://api.deepseek.com", "n"),
    ("ernie", "https://qianfan.baidubce.com/v2", "n"),
    ("github", "https://models.inference.ai.azure.com", "n"),
    ("groq", "https://api.groq.com/openai/v1", "n"),
    ("hunyuan", "https://api.hunyuan.cloud.tencent.com/v1", "n"),
    ("minimax", "https://api.minimax.chat/v1", "n"),
    ("mistral", "https://api.mistral.ai/v1", "n"),
    ("moonshot", "https://api.moonshot.cn/v1", "n"),
    ("openrouter", "https://openrouter.ai/api/v1", "y"),
    ("perplexity", "https://api.perplexity.ai", "n"),
    ("xai", "https://api.x.ai/v1", "n"),
    ("zhipuai", "https://open.bigmodel.cn/api/paas/v4", "n"),
    ("agnes", "https://apihub.agnes-ai.com/v1", "n"),
    ("amd", "https://developer.amd.com.cn/radeon/api/v1", "n"),
    ("modelscope", "https://api-inference.modelscope.cn/v1", "y"),
    ("siliconflow", "https://api.siliconflow.com/v1", "n"),
    ("siliconflow-cn", "https://api.siliconflow.cn/v1", "n"),
    ("volcengine", "https://ark.cn-beijing.volces.com/api/v3", "n"),
    ("nvidia", "https://integrate.api.nvidia.com/v1", "y"),
    ("fireworks-ai", "https://api.fireworks.ai/inference/v1/", "n"),
    ("huggingface", "https://router.huggingface.co/v1", "n"),
    ("baseten", "https://inference.baseten.co/v1", "n"),
    ("chutes", "https://llm.chutes.ai/v1", "y"),
    ("novita-ai", "https://api.novita.ai/openai", "y"),
    ("nebius", "https://api.tokenfactory.nebius.com/v1", "n"),
    ("friendli", "https://api.friendli.ai/serverless/v1", "y"),
    ("gmicloud", "https://api.gmi-serving.com/v1", "n"),
    ("digitalocean", "https://inference.do-ai.run/v1", "n"),
    ("hetzner", "https://inference.hetzner.com/api/v1", "n"),
    ("scaleway", "https://api.scaleway.ai/v1", "n"),
    ("ovhcloud", "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1", "y"),
    ("crusoe", "https://api.inference.crusoecloud.com/v1", "n"),
    ("vultr", "https://api.vultrinference.com/v1", "n"),
    ("io-net", "https://api.intelligence.io.solutions/api/v1", "n"),
    ("302ai", "https://api.302.ai/v1", "n"),
    ("poe", "https://api.poe.com/v1", "n"),
    ("llama", "https://api.llama.com/compat/v1/", "n"),
    ("lmstudio", "http://127.0.0.1:1234/v1", "n"),
    ("ollama-cloud", "https://ollama.com/v1", "y"),
    ("alibaba", "https://dashscope-intl.aliyuncs.com/compatible-mode/v1", "n"),
    ("alibaba-cn", "https://dashscope.aliyuncs.com/compatible-mode/v1", "n"),
    ("stepfun", "https://api.stepfun.com/v1", "n"),
    ("stepfun-ai", "https://api.stepfun.ai/v1", "n"),
    ("zai", "https://api.z.ai/api/paas/v4", "n"),
    ("zhipuai-coding-plan", "https://open.bigmodel.cn/api/coding/paas/v4", "n"),
    ("iflowcn", "https://apis.iflow.cn/v1", "n"),
    ("moonshotai", "https://api.moonshot.ai/v1", "n"),
    ("moonshotai-cn", "https://api.moonshot.cn/v1", "n"),
    ("qiniu-ai", "https://api.qnaigc.com/v1", "y"),
    ("sensenova", "https://token.sensenova.cn/v1", "n"),
    ("xiaomi", "https://api.xiaomimimo.com/v1", "n"),
    ("longcat", "https://api.longcat.chat/openai", "n"),
    ("tencent-tokenhub", "https://tokenhub.tencentmaas.com/v1", "n"),
    ("upstage", "https://api.upstage.ai/v1/solar", "n"),
    ("morph", "https://api.morphllm.com/v1", "n"),
    ("arcee", "https://api.arcee.ai/api/v1", "n"),
    ("inference", "https://inference.net/v1", "n"),
    ("hyper", "https://hyper.charm.land/v1", "y"),
    ("jiekou", "https://api.jiekou.ai/openai", "n"),
    ("jina", "https://api.jina.ai/v1", "n"),
    ("voyageai", "https://api.voyageai.com/v1", "n"),
    ("abacus", "https://routellm.abacus.ai/v1", "n"), // Abacus
    ("abliteration-ai", "https://api.abliteration.ai/v1", "n"), // abliteration.ai
    ("above", "https://api.above.dev/v1", "n"), // above.dev
    ("agentrouter", "https://agentrouter.org/v1", "n"), // AgentRouter
    ("ai-router", "https://api.ai-router.dev/v1", "n"), // AI-ROUTER
    ("aiand", "https://api.aiand.com/v1", "n"), // ai&
    ("ainetcafe", "https://microquickjs.com/v1", "n"), // ainetcafe
    ("aixy", "https://api.aixy-gateway.com/v1", "n"), // Aixy
    ("aki-io", "https://aki.io/v1", "n"), // AKI.IO
    ("alibaba-coding-plan", "https://coding-intl.dashscope.aliyuncs.com/v1", "n"), // Alibaba Coding Plan
    ("alibaba-coding-plan-cn", "https://coding.dashscope.aliyuncs.com/v1", "n"), // Alibaba Coding Plan (China)
    ("alibaba-token-plan", "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1", "n"), // Alibaba Token Plan
    ("alibaba-token-plan-cn", "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1", "n"), // Alibaba Token Plan (China)
    ("ambient", "https://api.ambient.xyz/v1", "n"), // Ambient
    ("anyapi", "https://api.anyapi.ai/v1", "n"), // AnyAPI
    ("atomic-chat", "http://127.0.0.1:1337/v1", "n"), // Atomic Chat
    ("auriko", "https://api.auriko.ai/v1", "n"), // Auriko
    ("bailing", "https://api.tbox.cn/api/llm/v1", "n"), // Bailing
    ("bee", "https://api.bee.heossi.com/bee", "n"), // Bee by HEOSSI
    ("berget", "https://api.berget.ai/v1", "n"), // Berget.AI
    ("blueclaw", "https://openai.blueclaw.network/v1", "n"), // Blue Claw
    ("bothub", "https://openai.bothub.ru/v1", "n"), // Bothub
    ("clarifai", "https://api.clarifai.com/v2/ext/openai/v1", "n"), // Clarifai
    ("claudinio", "https://api.claudin.io/v1", "n"), // Claudinio
    ("cline-pass", "https://api.cline.bot/api/v1", "n"), // ClinePass
    ("cloudferro-sherlock", "https://api-sherlock.cloudferro.com/openai/v1/", "n"), // CloudFerro Sherlock
    ("cloudflare-workers-ai", "https://api.cloudflare.com/client/v4/accounts/${CLOUDFLARE_ACCOUNT_ID}/ai/v1", "n"), // Cloudflare Workers AI
    ("coralbricks", "https://inference.coralbricks.ai/v1", "n"), // CoralBricks
    ("cortecs", "https://api.cortecs.ai/v1", "n"), // Cortecs
    ("crof", "https://crof.ai/v1", "n"), // CrofAI
    ("crossmodel", "https://api.crossmodel.ai/v1", "n"), // CrossModel
    ("daoxe", "https://daoxe.com/v1", "n"), // DaoXE
    ("databricks", "https://${DATABRICKS_HOST}/ai-gateway/mlflow/v1", "n"), // Databricks
    ("dinference", "https://api.dinference.com/v1", "n"), // DInference
    ("drun", "https://chat.d.run/v1", "n"), // D.Run (China)
    ("ebcloud", "https://maas-api.ebcloud.com/v1", "n"), // EBCloud
    ("echo", "https://echo.tracerml.ai/v1", "n"), // Echo
    ("edenai", "https://api.edenai.run/v3", "n"), // Eden AI
    ("empiriolabs", "https://api.empiriolabs.ai/v1", "n"), // EmpirioLabs AI
    ("engy", "https://api.engy.ai/v1", "n"), // engy
    ("evroc", "https://models.think.evroc.com/v1", "n"), // evroc
    ("fastrouter", "https://go.fastrouter.ai/api/v1", "n"), // FastRouter
    ("frogbot", "https://app.frogbot.ai/api/v1", "n"), // FrogBot
    ("github-copilot", "https://api.githubcopilot.com", "n"), // GitHub Copilot
    ("greenpt", "https://api.greenpt.ai/v1", "n"), // GreenPT
    ("helicone", "https://ai-gateway.helicone.ai/v1", "n"), // Helicone
    ("hpc-ai", "https://api.hpc-ai.com/inference/v1", "n"), // HPC-AI
    ("impossibl", "https://api.impossibl.com/v1", "n"), // Impossibl
    ("inception", "https://api.inceptionlabs.ai/v1/", "n"), // Inception
    ("inceptron", "https://api.inceptron.io/v1", "n"), // Inceptron
    ("inco", "https://api.inco.ai/v1", "n"), // Inco
    ("infer", "https://infer.flow7.org/v1", "n"), // Infer by Flow7
    ("inferx", "https://model.inferx.net/endpoints/v1", "n"), // InferX
    ("infomaniak", "https://api.infomaniak.com/2/ai/${INFOMANIAK_PRODUCT_ID}/openai/v1", "n"), // Infomaniak
    ("iteracompute", "https://api.iteracompute.com/v1", "n"), // IteraCompute
    ("jalapeno", "https://api.jalapeno-cloud.ai/v1", "n"), // Jalapeno Cloud
    ("kenari", "https://kenari.id/v1", "n"), // Kenari
    ("kilo", "https://api.kilo.ai/api/gateway", "n"), // Kilo Gateway
    ("kimi-code-plan-cn", "https://api.kimi.com/coding/v1", "n"), // Kimi For Coding (kimi.com)
    ("kimi-code-plan-global", "https://api.kimi.ai/coding/v1", "n"), // Kimi For Coding (kimi.ai)
    ("klokintegration", "https://api-gw.klok.ipaas.se/proxy/kloker-key/v1", "n"), // klokintegration.se
    ("kosmik", "https://api.koscompute.com/v1", "n"), // Kosmik Compute
    ("kuae-cloud-coding-plan", "https://coding-plan-endpoint.kuaecloud.net/v1", "n"), // KUAE Cloud Coding Plan
    ("lilac", "https://api.getlilac.com/v1", "n"), // Lilac
    ("llmgateway", "https://api.llmgateway.io/v1", "n"), // DevPass (LLM Gateway)
    ("llmgateway-providers", "https://api.llmgateway.io/v1", "n"), // LLM Gateway
    ("llmtech", "https://api.llmtech.eu/v1", "n"), // LLM Tech
    ("llmtr", "https://llmtr.com/v1", "n"), // LLMTR
    ("lucidquery", "https://api.lucidquery.com/v1", "n"), // LucidQuery
    ("lynkr", "http://127.0.0.1:8081/v1", "n"), // Lynkr
    ("meganova", "https://api.meganova.ai/v1", "n"), // Meganova
    ("melious", "https://api.melious.ai/v1", "n"), // Melious
    ("meta", "https://api.meta.ai/v1", "n"), // Meta
    ("mixlayer", "https://models.mixlayer.ai/v1", "n"), // Mixlayer
    ("moark", "https://moark.com/v1", "n"), // Moark
    ("modal", "https://inference.us-west.modal.direct/v1", "n"), // Modal
    ("model-oracle-ai", "https://api.modeloracle.com/api/v1", "n"), // Model Oracle AI
    ("modelis", "https://modelishub.com/v1", "n"), // Modelis
    ("nan", "https://api.nan.builders/v1", "n"), // NaN
    ("nano-gpt", "https://nano-gpt.com/api/v1", "n"), // NanoGPT
    ("nearai", "https://cloud-api.near.ai/v1", "n"), // NEAR AI Cloud
    ("neon", "${NEON_AI_GATEWAY_BASE_URL}/v1", "n"), // Neon
    ("neosmith", "https://router.neosmith.ai/v1", "n"), // NeoSmith
    ("neuralwatt", "https://api.neuralwatt.com/v1", "n"), // Neuralwatt
    ("nova", "https://api.nova.amazon.com/v1", "n"), // Nova
    ("oci", "https://inference.generativeai.us-chicago-1.oci.oraclecloud.com/openai/v1", "n"), // OCI Generative AI
    ("ofox", "https://api.ofox.ai/v1", "n"), // Ofox
    ("opencode", "https://opencode.ai/zen/v1", "n"), // OpenCode Zen
    ("opencode-go", "https://opencode.ai/zen/go/v1", "n"), // OpenCode Go
    ("openreason", "https://api.openreason.app/v1", "n"), // OpenReason
    ("opper", "https://api.opper.ai/v3/compat", "n"), // Opper
    ("orcarouter", "https://api.orcarouter.ai/v1", "n"), // OrcaRouter
    ("pareto", "https://api.paretoinference.com/v1", "n"), // Pareto Inference
    ("pendra", "https://api.pendra.ai/api/v1", "n"), // Pendra
    ("perplexity-agent", "https://api.perplexity.ai/v1", "n"), // Perplexity Agent
    ("pioneer", "https://api.fastino.ai/v1", "n"), // Pioneer
    ("poolside", "https://inference.poolside.ai/v1", "n"), // Poolside
    ("privatemode-ai", "http://localhost:8080/v1", "n"), // Privatemode AI
    ("qihang-ai", "https://api.qhaigc.net/v1", "n"), // QiHang
    ("regolo-ai", "https://api.regolo.ai/v1", "n"), // Regolo AI
    ("requesty", "https://router.requesty.ai/v1", "n"), // Requesty
    ("routing-run", "https://api.routing.run/v1", "n"), // routing.run
    ("runinfra", "https://api.runinfra.ai/v1", "n"), // RunInfra
    ("sakana", "https://api.sakana.ai/v1", "n"), // Sakana AI
    ("sarvam", "https://api.sarvam.ai/v1", "n"), // Sarvam AI
    ("scnet-token-plan", "https://api.scnet.cn/api/llm/v1", "n"), // SCNet Token Plan
    ("scx-ai", "https://api.scx.ai/v1", "n"), // SCX.ai
    ("snowflake-cortex", "https://${SNOWFLAKE_ACCOUNT}.snowflakecomputing.com/api/v2/cortex/v1", "n"), // Snowflake Cortex
    ("stackit", "https://api.openai-compat.model-serving.eu01.onstackit.cloud/v1", "n"), // STACKIT
    ("stepfun-ai-step-plan", "https://api.stepfun.ai/step_plan/v1", "n"), // StepFun Step Plan (Global)
    ("stepfun-step-plan", "https://api.stepfun.com/step_plan/v1", "n"), // StepFun Step Plan (China)
    ("submodel", "https://llm.submodel.ai/v1", "n"), // submodel
    ("synthetic", "https://api.synthetic.new/openai/v1", "n"), // Synthetic
    ("tempr", "https://api.temprhq.io/v1", "n"), // Tempr Gateway
    ("tencent-coding-plan", "https://api.lkeap.cloud.tencent.com/coding/v3", "n"), // Tencent Coding Plan (China)
    ("tencent-token-plan", "https://api.lkeap.cloud.tencent.com/plan/v3", "n"), // Tencent Token Plan
    ("tensorx", "https://api.tensorx.ai/v1", "n"), // TensorX
    ("the-grid-ai", "https://api.thegrid.ai/v1", "n"), // The Grid AI
    ("tinfoil", "https://inference.tinfoil.sh/v1", "n"), // Tinfoil
    ("tokengo", "https://api.tokengo.com/v1", "n"), // TokenGo
    ("tokenrouter", "https://api.tokenrouter.com/v1", "n"), // TokenRouter
    ("trustedrouter", "https://api.trustedrouter.com/v1", "n"), // TrustedRouter
    ("umans-ai", "https://api.code.umans.ai/v1", "n"), // Umans AI
    ("umans-ai-coding-plan", "https://api.code.umans.ai/v1", "n"), // Umans AI Coding Plan
    ("unorouter", "https://api.unorouter.com/v1", "n"), // UnoRouter
    ("vancine", "https://vancine.com/v1", "n"), // Vancine
    ("vispark", "https://api.lab.vispark.in/v1", "n"), // Vispark
    ("vivgrid", "https://api.vivgrid.com/v1", "n"), // Vivgrid
    ("volcengine-coding-plan", "https://ark.cn-beijing.volces.com/api/coding/v3", "n"), // Volcengine Ark Coding Plan
    ("wafer.ai", "https://pass.wafer.ai/v1", "n"), // Wafer
    ("wallaby", "https://api.wallabytoken.com/v1", "n"), // Wallaby
    ("wandb", "https://api.inference.wandb.ai/v1", "n"), // CoreWeave
    ("xiaomi-token-plan-ams", "https://token-plan-ams.xiaomimimo.com/v1", "n"), // Xiaomi Token Plan (Europe)
    ("xiaomi-token-plan-cn", "https://token-plan-cn.xiaomimimo.com/v1", "n"), // Xiaomi Token Plan (China)
    ("xiaomi-token-plan-sgp", "https://token-plan-sgp.xiaomimimo.com/v1", "n"), // Xiaomi Token Plan (Singapore)
    ("xpersona", "https://www.xpersona.co/v1", "n"), // Xpersona
    ("zai-coding-plan", "https://api.z.ai/api/coding/paas/v4", "n"), // Z.AI Coding Plan
    ("zeldoc", "https://api.zeldoc.ai/v1", "n"), // Zeldoc
    ("zenifra", "https://ai.zenifra.com/v1", "n"), // Zenifra
    ("zenmux", "https://zenmux.ai/api/v1", "n"), // ZenMux
];

pub fn provider_info(name: &str) -> Option<(&'static str, &'static str)> {
    PROVIDERS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, b, o)| (*b, *o))
}
