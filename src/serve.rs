use crate::{client::*, config::*, function::*, rag::*, utils::*};

use anyhow::{anyhow, bail, Result};
use bytes::Bytes;
use chrono::{Timelike, Utc};
use futures_util::StreamExt;
use http::{Method, Response, StatusCode};
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Frame, Incoming},
    service::service_fn,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};
use tokio_graceful::Shutdown;
use tokio_stream::wrappers::UnboundedReceiverStream;

const DEFAULT_MODEL_NAME: &str = "default";
const PLAYGROUND_HTML: &[u8] = include_bytes!("../assets/playground.html");
const ARENA_HTML: &[u8] = include_bytes!("../assets/arena.html");

type AppResponse = Response<BoxBody<Bytes, Infallible>>;

pub async fn run(config: GlobalConfig, addr: Option<String>) -> Result<()> {
    let addr = match addr {
        Some(addr) => {
            if let Ok(port) = addr.parse::<u16>() {
                format!("127.0.0.1:{port}")
            } else if let Ok(ip) = addr.parse::<IpAddr>() {
                format!("{ip}:8000")
            } else {
                addr
            }
        }
        None => config.read().serve_addr(),
    };
    let server = Arc::new(Server::new(&config));
    let listener = TcpListener::bind(&addr).await?;
    let stop_server = server.run(listener).await?;
    println!("Chat Completions API: http://{addr}/v1/chat/completions");
    println!("Responses API:        http://{addr}/v1/responses");
    println!("Embeddings API:       http://{addr}/v1/embeddings");
    println!("Rerank API:           http://{addr}/v1/rerank");
    println!("LLM Playground:       http://{addr}/playground");
    println!("LLM Arena:            http://{addr}/arena?num=2");
    shutdown_signal().await;
    let _ = stop_server.send(());
    Ok(())
}

struct Server {
    config: Config,
    models: Vec<Value>,
    roles: Vec<Role>,
    rags: Vec<String>,
}

impl Server {
    fn new(config: &GlobalConfig) -> Self {
        let mut config = config.read().clone();
        config.functions = Functions::default();
        let mut models = list_all_models(&config);
        let mut default_model = config.model.clone();
        default_model.data_mut().name = DEFAULT_MODEL_NAME.into();
        models.insert(0, &default_model);
        let models: Vec<Value> = models
            .into_iter()
            .enumerate()
            .map(|(i, model)| {
                let id = if i == 0 {
                    DEFAULT_MODEL_NAME.into()
                } else {
                    model.id()
                };
                let mut value = json!(model.data());
                if let Some(value_obj) = value.as_object_mut() {
                    value_obj.insert("id".into(), id.into());
                    value_obj.insert("object".into(), "model".into());
                    value_obj.insert("owned_by".into(), model.client_name().into());
                    value_obj.remove("name");
                }
                value
            })
            .collect();
        Self {
            config,
            models,
            roles: Config::all_roles(),
            rags: Config::list_rags(),
        }
    }

    async fn run(self: Arc<Self>, listener: TcpListener) -> Result<oneshot::Sender<()>> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let shutdown = Shutdown::new(async { rx.await.unwrap_or_default() });
            let guard = shutdown.guard_weak();

            loop {
                tokio::select! {
                    res = listener.accept() => {
                        let Ok((cnx, _)) = res else {
                            continue;
                        };

                        let stream = TokioIo::new(cnx);
                        let server = self.clone();
                        shutdown.spawn_task(async move {
                            let hyper_service = service_fn(move |request: hyper::Request<Incoming>| {
                                server.clone().handle(request)
                            });
                            let _ = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                                .serve_connection_with_upgrades(stream, hyper_service)
                                .await;
                        });
                    }
                    _ = guard.cancelled() => {
                        break;
                    }
                }
            }
        });
        Ok(tx)
    }

    async fn handle(
        self: Arc<Self>,
        req: hyper::Request<Incoming>,
    ) -> std::result::Result<AppResponse, hyper::Error> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let path = uri.path();

        if method == Method::OPTIONS {
            let mut res = Response::default();
            *res.status_mut() = StatusCode::NO_CONTENT;
            set_cors_header(&mut res);
            return Ok(res);
        }

        let mut status = StatusCode::OK;
        let res = if path == "/v1/chat/completions" {
                self.chat_completions(req).await
            } else if path == "/v1/responses" {
                self.responses(req).await
            } else if path == "/v1/embeddings" {
            self.embeddings(req).await
        } else if path == "/v1/rerank" {
            self.rerank(req).await
        } else if path == "/v1/models" {
            self.list_models()
        } else if path == "/v1/roles" {
            self.list_roles()
        } else if path == "/v1/rags" {
            self.list_rags()
        } else if path == "/v1/rags/search" {
            self.search_rag(req).await
        } else if path == "/playground" || path == "/playground.html" {
            self.playground_page()
        } else if path == "/arena" || path == "/arena.html" {
            self.arena_page()
        } else {
            status = StatusCode::NOT_FOUND;
            Err(anyhow!("Not Found"))
        };
        let mut res = match res {
            Ok(res) => {
                info!("{method} {uri} {}", status.as_u16());
                res
            }
            Err(err) => {
                if status == StatusCode::OK {
                    status = StatusCode::BAD_REQUEST;
                }
                error!("{method} {uri} {} {err}", status.as_u16());
                ret_err(err)
            }
        };
        *res.status_mut() = status;
        set_cors_header(&mut res);
        Ok(res)
    }

    fn playground_page(&self) -> Result<AppResponse> {
        let res = Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(PLAYGROUND_HTML)).boxed())?;
        Ok(res)
    }

    fn arena_page(&self) -> Result<AppResponse> {
        let res = Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(ARENA_HTML)).boxed())?;
        Ok(res)
    }

    fn list_models(&self) -> Result<AppResponse> {
        let data = json!({ "data": self.models });
        let res = Response::builder()
            .header("Content-Type", "application/json; charset=utf-8")
            .body(Full::new(Bytes::from(data.to_string())).boxed())?;
        Ok(res)
    }

    fn list_roles(&self) -> Result<AppResponse> {
        let data = json!({ "data": self.roles });
        let res = Response::builder()
            .header("Content-Type", "application/json; charset=utf-8")
            .body(Full::new(Bytes::from(data.to_string())).boxed())?;
        Ok(res)
    }

    fn list_rags(&self) -> Result<AppResponse> {
        let data = json!({ "data": self.rags });
        let res = Response::builder()
            .header("Content-Type", "application/json; charset=utf-8")
            .body(Full::new(Bytes::from(data.to_string())).boxed())?;
        Ok(res)
    }

    async fn search_rag(&self, req: hyper::Request<Incoming>) -> Result<AppResponse> {
        let req_body = req.collect().await?.to_bytes();
        let req_body: Value = serde_json::from_slice(&req_body)
            .map_err(|err| anyhow!("Invalid request json, {err}"))?;

        debug!("search rag request: {req_body}");
        let SearchRagReqBody { name, input } = serde_json::from_value(req_body)
            .map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let config = Arc::new(RwLock::new(self.config.clone()));

        let abort_signal = create_abort_signal();

        let rag_path = config.read().rag_file(&name);
        let rag = Rag::load(&config, &name, &rag_path)?;

        let rag_result = Config::search_rag(&config, &rag, &input, abort_signal).await?;

        let data = json!({ "data": rag_result });
        let res = Response::builder()
            .header("Content-Type", "application/json; charset=utf-8")
            .body(Full::new(Bytes::from(data.to_string())).boxed())?;
        Ok(res)
    }

    async fn chat_completions(&self, req: hyper::Request<Incoming>) -> Result<AppResponse> {
        let req_body = req.collect().await?.to_bytes();
        let req_body: Value = serde_json::from_slice(&req_body)
            .map_err(|err| anyhow!("Invalid request json, {err}"))?;

        debug!("chat completions request: {req_body}");
        let req_body = serde_json::from_value(req_body)
            .map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let ChatCompletionsReqBody {
            model,
            messages,
            temperature,
            top_p,
            max_tokens,
            stream,
            tools,
        } = req_body;

        let mut messages =
            parse_messages(messages).map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let functions = parse_tools(tools).map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let config = self.config.clone();

        let default_model = config.model.clone();

        let config = Arc::new(RwLock::new(config));

        let (model_name, change) = if model == DEFAULT_MODEL_NAME {
            (default_model.id(), true)
        } else if default_model.id() == model {
            (model, false)
        } else {
            (model, true)
        };

        if change {
            config.write().set_model(&model_name)?;
        }

        let mut client = init_client(&config, None)?;
        if max_tokens.is_some() {
            client.model_mut().set_max_tokens(max_tokens, true);
        }
        let abort_signal = create_abort_signal();
        let http_client = client.build_client()?;

        let completion_id = generate_completion_id();
        let created = Utc::now().timestamp();

        patch_messages(&mut messages, client.model());

        let data: ChatCompletionsData = ChatCompletionsData {
            messages,
            temperature,
            top_p,
            functions,
            stream,
        };

        if stream {
            let (tx, mut rx) = unbounded_channel();
            tokio::spawn(async move {
                let is_first = Arc::new(AtomicBool::new(true));
                let (sse_tx, sse_rx) = unbounded_channel();
                let mut handler = SseHandler::new(sse_tx, abort_signal);
                async fn map_event(
                    mut sse_rx: UnboundedReceiver<SseEvent>,
                    tx: &UnboundedSender<ResEvent>,
                    is_first: Arc<AtomicBool>,
                ) {
                    while let Some(reply_event) = sse_rx.recv().await {
                        if is_first.load(Ordering::SeqCst) {
                            let _ = tx.send(ResEvent::First(None));
                            is_first.store(false, Ordering::SeqCst)
                        }
                        match reply_event {
                            SseEvent::Text(text) => {
                                let _ = tx.send(ResEvent::Text(text));
                            }
                            SseEvent::Done => {
                                let _ = tx.send(ResEvent::Done);
                                sse_rx.close();
                            }
                        }
                    }
                }
                async fn chat_completions(
                    client: &dyn Client,
                    http_client: &reqwest::Client,
                    handler: &mut SseHandler,
                    mut data: ChatCompletionsData,
                    tx: &UnboundedSender<ResEvent>,
                    is_first: Arc<AtomicBool>,
                ) {
                    if client.model().no_stream() {
                        data.stream = false;
                        let ret = client.chat_completions_inner(http_client, data).await;
                        match ret {
                            Ok(output) => {
                                let ChatCompletionsOutput {
                                    text, tool_calls, ..
                                } = output;
                                let _ = tx.send(ResEvent::First(None));
                                is_first.store(false, Ordering::SeqCst);
                                let _ = tx.send(ResEvent::Text(text));
                                if !tool_calls.is_empty() {
                                    let _ = tx.send(ResEvent::ToolCalls(tool_calls));
                                }
                            }
                            Err(err) => {
                                let _ = tx.send(ResEvent::First(Some(format!("{err:?}"))));
                                is_first.store(false, Ordering::SeqCst)
                            }
                        };
                    } else {
                        let ret = client
                            .chat_completions_streaming_inner(http_client, handler, data)
                            .await;
                        let first = match ret {
                            Ok(()) => None,
                            Err(err) => Some(format!("{err:?}")),
                        };
                        if is_first.load(Ordering::SeqCst) {
                            let _ = tx.send(ResEvent::First(first));
                            is_first.store(false, Ordering::SeqCst)
                        }
                        let tool_calls = handler.tool_calls().to_vec();
                        if !tool_calls.is_empty() {
                            let _ = tx.send(ResEvent::ToolCalls(tool_calls));
                        }
                    }
                    handler.done();
                }
                tokio::join!(
                    map_event(sse_rx, &tx, is_first.clone()),
                    chat_completions(
                        client.as_ref(),
                        &http_client,
                        &mut handler,
                        data,
                        &tx,
                        is_first
                    ),
                );
            });

            let first_event = rx.recv().await;

            if let Some(ResEvent::First(Some(err))) = first_event {
                bail!("{err}");
            }

            let shared: Arc<(String, String, i64, AtomicBool)> =
                Arc::new((completion_id, model_name, created, AtomicBool::new(false)));
            let stream = UnboundedReceiverStream::new(rx);
            let stream = stream.filter_map(move |res_event| {
                let shared = shared.clone();
                async move {
                    let (completion_id, model, created, has_tool_calls) = shared.as_ref();
                    match res_event {
                        ResEvent::Text(text) => {
                            Some(Ok(create_text_frame(completion_id, model, *created, &text)))
                        }
                        ResEvent::ToolCalls(tool_calls) => {
                            has_tool_calls.store(true, Ordering::SeqCst);
                            Some(Ok(create_tool_calls_frame(
                                completion_id,
                                model,
                                *created,
                                &tool_calls,
                            )))
                        }
                        ResEvent::Done => Some(Ok(create_done_frame(
                            completion_id,
                            model,
                            *created,
                            has_tool_calls.load(Ordering::SeqCst),
                        ))),
                        _ => None,
                    }
                }
            });
            let res = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(BodyExt::boxed(StreamBody::new(stream)))?;
            Ok(res)
        } else {
            let output = client.chat_completions_inner(&http_client, data).await?;
            let res = Response::builder()
                .header("Content-Type", "application/json")
                .body(
                    Full::new(ret_non_stream(
                        &completion_id,
                        &model_name,
                        created,
                        &output,
                    ))
                    .boxed(),
                )?;
            Ok(res)
        }
    }

    /// OpenAI `/v1/responses` compatible endpoint: accepts a CreateResponse body,
    /// executes it against the configured client, and returns a `Response` object
    /// (non-stream) or a Responses SSE event stream (stream).
    async fn responses(&self, req: hyper::Request<Incoming>) -> Result<AppResponse> {
        let req_body = req.collect().await?.to_bytes();
        let req_body: Value = serde_json::from_slice(&req_body)
            .map_err(|err| anyhow!("Invalid request json, {err}"))?;

        debug!("responses request: {req_body}");
        let req_body: ResponsesReqBody = serde_json::from_value(req_body)
            .map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let ResponsesReqBody {
            model,
            input,
            instructions,
            temperature,
            top_p,
            max_output_tokens,
            stream,
            tools,
        } = req_body;

        let messages = parse_responses_input(input, instructions)
            .map_err(|err| anyhow!("Invalid request body, {err}"))?;
        let functions = parse_tools(tools).map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let config = self.config.clone();
        let default_model = config.model.clone();

        let config = Arc::new(RwLock::new(config));

        let (model_name, change) = if model == DEFAULT_MODEL_NAME {
            (default_model.id(), true)
        } else if default_model.id() == model {
            (model, false)
        } else {
            (model, true)
        };

        if change {
            config.write().set_model(&model_name)?;
        }

        let mut client = init_client(&config, None)?;
        if let Some(max_output_tokens) = max_output_tokens {
            client.model_mut().set_max_tokens(Some(max_output_tokens), true);
        }
        let abort_signal = create_abort_signal();
        let http_client = client.build_client()?;

        let response_id = generate_completion_id();
        let created = Utc::now().timestamp();

        let mut messages = messages;
        patch_messages(&mut messages, client.model());

        let data: ChatCompletionsData = ChatCompletionsData {
            messages,
            temperature,
            top_p,
            functions,
            stream,
        };

        if stream {
            let (tx, mut rx) = unbounded_channel();
            tokio::spawn(async move {
                let (sse_tx, sse_rx) = unbounded_channel();
                let mut handler = SseHandler::new(sse_tx, abort_signal);
                async fn map_event(
                    mut sse_rx: UnboundedReceiver<SseEvent>,
                    tx: &UnboundedSender<ResEvent>,
                ) {
                    let mut started = false;
                    while let Some(reply_event) = sse_rx.recv().await {
                        match reply_event {
                            SseEvent::Text(text) => {
                                if !started {
                                    let _ = tx.send(ResEvent::First(None));
                                    started = true;
                                }
                                let _ = tx.send(ResEvent::Text(text));
                            }
                            SseEvent::Done => {
                                if started {
                                    let _ = tx.send(ResEvent::MessageDone);
                                }
                                let _ = tx.send(ResEvent::Done);
                                sse_rx.close();
                            }
                        }
                    }
                }
                async fn chat_completions(
                    client: &dyn Client,
                    http_client: &reqwest::Client,
                    handler: &mut SseHandler,
                    data: ChatCompletionsData,
                    tx: &UnboundedSender<ResEvent>,
                ) {
                    let ret = client
                        .chat_completions_streaming_inner(http_client, handler, data)
                        .await;
                    if let Err(err) = ret {
                        let _ = tx.send(ResEvent::Error(format!("{err:?}")));
                    }
                    // Send Done while map_event is still in the join! to release it.
                    handler.done();
                }
                let (_, ()) = tokio::join!(
                    map_event(sse_rx, &tx),
                    chat_completions(client.as_ref(), &http_client, &mut handler, data, &tx),
                );
            });

            let shared: Arc<(String, String, i64)> =
                Arc::new((response_id, model_name, created));
            let stream = UnboundedReceiverStream::new(rx);
            let first_text = Arc::new(AtomicBool::new(true));
            let stream = stream.filter_map(move |res_event| {
                let shared = shared.clone();
                let first_text = first_text.clone();
                async move {
                    let (response_id, model, created) = shared.as_ref();
                    match res_event {
                        ResEvent::First(_) => Some(Ok(create_responses_created_frame(
                            response_id, model, *created,
                        ))),
                        ResEvent::Text(text) => {
                            let is_first = first_text.swap(false, Ordering::SeqCst);
                            Some(Ok(create_responses_text_frame(
                                response_id, model, *created, &text, is_first,
                            )))
                        }
                        ResEvent::ToolCalls(tool_calls) => {
                            Some(Ok(create_responses_tool_calls_frame(
                                response_id,
                                model,
                                *created,
                                &tool_calls,
                            )))
                        }
                        ResEvent::MessageDone => Some(Ok(create_responses_message_done_frame(
                            response_id, model, *created,
                        ))),
                        ResEvent::Error(err) => Some(Ok(create_responses_error_frame(
                            response_id, model, *created, &err,
                        ))),
                        ResEvent::Done => Some(Ok(create_responses_completed_frame(
                            response_id, model, *created,
                        ))),
                        _ => None,
                    }
                }
            });
            let res = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(BodyExt::boxed(StreamBody::new(stream)))?;
            Ok(res)
        } else {
            let output = client.chat_completions_inner(&http_client, data).await?;
            let res = Response::builder()
                .header("Content-Type", "application/json")
                .body(
                    Full::new(ret_responses_non_stream(
                        &response_id,
                        &model_name,
                        created,
                        &output,
                    ))
                    .boxed(),
                )?;
            Ok(res)
        }
    }

    async fn embeddings(&self, req: hyper::Request<Incoming>) -> Result<AppResponse> {
        let req_body = req.collect().await?.to_bytes();
        let req_body: Value = serde_json::from_slice(&req_body)
            .map_err(|err| anyhow!("Invalid request json, {err}"))?;

        debug!("embeddings request: {req_body}");
        let req_body = serde_json::from_value(req_body)
            .map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let EmbeddingsReqBody {
            input,
            model: embedding_model_id,
        } = req_body;

        let config = Arc::new(RwLock::new(self.config.clone()));

        let embedding_model =
            Model::retrieve_model(&config.read(), &embedding_model_id, ModelType::Embedding)?;

        let texts = match input {
            EmbeddingsReqBodyInput::Single(v) => vec![v],
            EmbeddingsReqBodyInput::Multiple(v) => v,
        };
        let client = init_client(&config, Some(embedding_model))?;
        let data = client
            .embeddings(&EmbeddingsData {
                query: false,
                texts,
            })
            .await?;
        let data: Vec<_> = data
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                json!({
                        "object": "embedding",
                        "embedding": v,
                        "index": i,
                })
            })
            .collect();
        let output = json!({
            "object": "list",
            "data": data,
            "model": embedding_model_id,
            "usage": {
                "prompt_tokens": 0,
                "total_tokens": 0,
            }
        });
        let res = Response::builder()
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(output.to_string())).boxed())?;
        Ok(res)
    }

    async fn rerank(&self, req: hyper::Request<Incoming>) -> Result<AppResponse> {
        let req_body = req.collect().await?.to_bytes();
        let req_body: Value = serde_json::from_slice(&req_body)
            .map_err(|err| anyhow!("Invalid request json, {err}"))?;

        debug!("rerank request: {req_body}");
        let req_body = serde_json::from_value(req_body)
            .map_err(|err| anyhow!("Invalid request body, {err}"))?;

        let RerankReqBody {
            model: reranker_model_id,
            documents,
            query,
            top_n,
        } = req_body;

        let top_n = top_n.unwrap_or(documents.len());

        let config = Arc::new(RwLock::new(self.config.clone()));

        let reranker_model =
            Model::retrieve_model(&config.read(), &reranker_model_id, ModelType::Reranker)?;

        let client = init_client(&config, Some(reranker_model))?;
        let data = client
            .rerank(&RerankData {
                query,
                documents: documents.clone(),
                top_n,
            })
            .await?;

        let results: Vec<_> = data
            .into_iter()
            .map(|v| {
                json!({
                    "index": v.index,
                    "relevance_score": v.relevance_score,
                    "document": documents.get(v.index).map(|v| json!(v)).unwrap_or_default(),
                })
            })
            .collect();
        let output = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "results": results,
        });
        let res = Response::builder()
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(output.to_string())).boxed())?;
        Ok(res)
    }
}

#[derive(Debug, Deserialize)]
struct SearchRagReqBody {
    name: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsReqBody {
    model: String,
    messages: Vec<Value>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_tokens: Option<isize>,
    #[serde(default)]
    stream: bool,
    tools: Option<Vec<Value>>,
}

/// CreateResponse body for the /v1/responses endpoint (subset of fields).
#[derive(Debug, Deserialize)]
struct ResponsesReqBody {
    model: String,
    /// A text string or a list of input items (EasyInputMessage / function_call / function_call_output).
    input: Value,
    instructions: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<isize>,
    #[serde(default)]
    stream: bool,
    tools: Option<Vec<Value>>,
}

/// Parse the Responses API `input` (string or item list) plus `instructions`
/// into internal messages.
fn parse_responses_input(input: Value, instructions: Option<String>) -> Result<Vec<Message>> {
    let mut output = vec![];
    if let Some(instructions) = instructions {
        output.push(Message::new(
            MessageRole::System,
            MessageContent::Text(instructions),
        ));
    }
    match input {
        Value::String(text) => output.push(Message::new(
            MessageRole::User,
            MessageContent::Text(text),
        )),
        Value::Array(items) => {
            // Buffer function_call -> following function_call_output items into
            // a single assistant ToolCalls message (internal representation).
            let mut pending_calls: Vec<(Option<String>, String, Value)> = vec![];
            for (i, item) in items.into_iter().enumerate() {
                let err = || anyhow!("Failed to parse '.input[{i}]'");
                let item_type = item.get("type").and_then(|v| v.as_str());
                match item_type {
                    Some("function_call") => {
                        let name = item["name"].as_str().ok_or_else(err)?;
                        let arguments = item["arguments"]
                            .as_str()
                            .unwrap_or("{}")
                            .parse()
                            .map_err(|_| err())?;
                        pending_calls.push((
                            item["call_id"].as_str().map(|v| v.to_string()),
                            name.to_string(),
                            arguments,
                        ));
                    }
                    Some("function_call_output") => {
                        let call_id = item["call_id"].as_str().map(|v| v.to_string());
                        let output_text = match &item["output"] {
                            Value::String(s) => s.clone(),
                            v => v.to_string(),
                        };
                        // Find and complete the matching call.
                        let idx = pending_calls
                            .iter()
                            .position(|(id, _, _)| id.as_deref() == call_id.as_deref())
                            .ok_or_else(err)?;
                        let (id, name, arguments) = pending_calls.remove(idx);
                        // Flush previous completed pair (simplified: one pair per message)
                        let tool_result = ToolResult::new(
                            ToolCall::new(name, arguments, id),
                            serde_json::from_str::<Value>(&output_text)
                                .unwrap_or_else(|_| Value::String(output_text.clone())),
                        );
                        output.push(Message::new(
                            MessageRole::Assistant,
                            MessageContent::ToolCalls(MessageContentToolCalls::new(
                                vec![tool_result],
                                String::new(),
                            )),
                        ));
                    }
                    _ => {
                        // EasyInputMessage: {role, content} where content is a string
                        // or a list of {type: "input_text"/"text", text} parts.
                        if pending_calls.len() == 1 {
                            // flush a dangling single function_call as its own message later
                        }
                        let role = item["role"].as_str().ok_or_else(err)?;
                        let role = match role {
                            "system" | "developer" => MessageRole::System,
                            "assistant" => MessageRole::Assistant,
                            "user" => MessageRole::User,
                            _ => return Err(err()),
                        };
                        let text = match item.get("content") {
                            Some(Value::String(s)) => s.clone(),
                            Some(Value::Array(parts)) => parts
                                .iter()
                                .filter_map(|part| part["text"].as_str())
                                .collect::<Vec<_>>()
                                .join(""),
                            _ => String::new(),
                        };
                        if !pending_calls.is_empty() {
                            // assistant text following function calls: emit calls first
                            let calls = std::mem::take(&mut pending_calls);
                            let tool_calls_msg = MessageContent::ToolCalls(
                                MessageContentToolCalls::new(
                                    calls
                                        .into_iter()
                                        .map(|(id, name, arguments)| {
                                            ToolResult::new(
                                                ToolCall::new(name, arguments, id),
                                                Value::Null,
                                            )
                                        })
                                        .collect(),
                                    String::new(),
                                ),
                            );
                            output.push(Message::new(MessageRole::Assistant, tool_calls_msg));
                        }
                        output.push(Message::new(role, MessageContent::Text(text)));
                    }
                }
            }
            // Flush any dangling function calls.
            if !pending_calls.is_empty() {
                let calls = pending_calls;
                let tool_calls_msg = MessageContent::ToolCalls(MessageContentToolCalls::new(
                    calls
                        .into_iter()
                        .map(|(id, name, arguments)| {
                            ToolResult::new(ToolCall::new(name, arguments, id), Value::Null)
                        })
                        .collect(),
                    String::new(),
                ));
                output.push(Message::new(MessageRole::Assistant, tool_calls_msg));
            }
        }
        _ => bail!("Invalid input"),
    }
    Ok(output)
}

#[derive(Debug, Deserialize)]
struct EmbeddingsReqBody {
    input: EmbeddingsReqBodyInput,
    model: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EmbeddingsReqBodyInput {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct RerankReqBody {
    documents: Vec<String>,
    query: String,
    model: String,
    top_n: Option<usize>,
}

#[derive(Debug)]
enum ResEvent {
    First(Option<String>),
    Text(String),
    ToolCalls(Vec<ToolCall>),
    MessageDone,
    Error(String),
    Done,
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler")
}

fn generate_completion_id() -> String {
    let random_id = chrono::Utc::now().nanosecond();
    format!("chatcmpl-{random_id}")
}

fn set_cors_header(res: &mut AppResponse) {
    res.headers_mut().insert(
        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        hyper::header::HeaderValue::from_static("*"),
    );
    res.headers_mut().insert(
        hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
        hyper::header::HeaderValue::from_static("GET,POST,PUT,PATCH,DELETE"),
    );
    res.headers_mut().insert(
        hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
        hyper::header::HeaderValue::from_static("Content-Type,Authorization"),
    );
}

fn create_text_frame(id: &str, model: &str, created: i64, content: &str) -> Frame<Bytes> {
    let delta = if content.is_empty() {
        json!({ "role": "assistant", "content": content })
    } else {
        json!({ "content": content })
    };
    let choice = json!({
        "index": 0,
        "delta": delta,
        "finish_reason": null,
    });
    let value = build_chat_completion_chunk_json(id, model, created, &choice);
    Frame::data(Bytes::from(format!("data: {value}\n\n")))
}

fn create_tool_calls_frame(
    id: &str,
    model: &str,
    created: i64,
    tool_calls: &[ToolCall],
) -> Frame<Bytes> {
    let chunks = tool_calls
        .iter()
        .enumerate()
        .flat_map(|(i, call)| {
            let choice1 = json!({
              "index": 0,
              "delta": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                  {
                    "index": i,
                    "id": call.id,
                    "type": "function",
                    "function": {
                      "name": call.name,
                      "arguments": ""
                    }
                  }
                ]
              },
              "finish_reason": null
            });
            let choice2 = json!({
              "index": 0,
              "delta": {
                "tool_calls": [
                  {
                    "index": i,
                    "function": {
                      "arguments": call.arguments.to_string(),
                    }
                  }
                ]
              },
              "finish_reason": null
            });
            vec![
                build_chat_completion_chunk_json(id, model, created, &choice1),
                build_chat_completion_chunk_json(id, model, created, &choice2),
            ]
        })
        .map(|v| format!("data: {v}\n\n"))
        .collect::<Vec<String>>()
        .join("");
    Frame::data(Bytes::from(chunks))
}

fn create_done_frame(id: &str, model: &str, created: i64, has_tool_calls: bool) -> Frame<Bytes> {
    let finish_reason = if has_tool_calls { "tool_calls" } else { "stop" };
    let choice = json!({
        "index": 0,
        "delta": {},
        "finish_reason": finish_reason,
    });
    let value = build_chat_completion_chunk_json(id, model, created, &choice);
    Frame::data(Bytes::from(format!("data: {value}\n\ndata: [DONE]\n\n")))
}

fn build_chat_completion_chunk_json(id: &str, model: &str, created: i64, choice: &Value) -> Value {
    json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
        "choices": [choice],
    })
}

// ---- Responses API frame builders ----

fn build_response_object_json(id: &str, model: &str, created: i64, status: &str) -> Value {
    json!({
        "id": id,
        "object": "response",
        "created_at": created,
        "status": status,
        "model": model,
        "output": [],
        "usage": {
            "input_tokens": 0,
            "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
            "output_tokens": 0,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": 0,
        },
        "error": null,
        "incomplete_details": null,
    })
}

fn create_responses_text_frame(
    id: &str,
    _model: &str,
    _created: i64,
    text: &str,
    with_item_added: bool,
) -> Frame<Bytes> {
    let mut buf = String::new();
    if with_item_added {
        let item = json!({
            "type": "response.output_item.added",
            "output_index": 0,
            "item": {
                "type": "message",
                "id": format!("msg_{id}"),
                "role": "assistant",
                "status": "in_progress",
                "content": [],
            },
        });
        let part = json!({
            "type": "response.content_part.added",
            "item_id": format!("msg_{id}"),
            "output_index": 0,
            "content_index": 0,
            "part": {"type": "output_text", "text": "", "annotations": []},
        });
        buf.push_str(&format!("data: {item}\n\ndata: {part}\n\n"));
    }
    let value = json!({
        "type": "response.output_text.delta",
        "item_id": format!("msg_{id}"),
        "output_index": 0,
        "content_index": 0,
        "delta": text,
    });
    buf.push_str(&format!("data: {value}\n\n"));
    Frame::data(Bytes::from(buf))
}

fn create_responses_created_frame(id: &str, model: &str, created: i64) -> Frame<Bytes> {
    let value = build_response_object_json(id, model, created, "in_progress");
    let event = json!({
        "type": "response.created",
        "response": value,
    });
    Frame::data(Bytes::from(format!("data: {event}\n\n")))
}

fn create_responses_item_added_frame(id: &str, model: &str, created: i64) -> Frame<Bytes> {
    let event = json!({
        "type": "response.output_item.added",
        "output_index": 0,
        "item": {
            "type": "message",
            "id": format!("msg_{id}"),
            "role": "assistant",
            "status": "in_progress",
            "content": [],
        },
    });
    let part = json!({
        "type": "response.content_part.added",
        "item_id": format!("msg_{id}"),
        "output_index": 0,
        "content_index": 0,
        "part": {"type": "output_text", "text": "", "annotations": []},
    });
    let _ = (model, created);
    Frame::data(Bytes::from(format!(
        "data: {event}\n\ndata: {part}\n\n"
    )))
}

fn create_responses_message_done_frame(id: &str, _model: &str, _created: i64) -> Frame<Bytes> {
    let item = json!({
        "type": "response.output_item.done",
        "output_index": 0,
        "item": {
            "type": "message",
            "id": format!("msg_{id}"),
            "role": "assistant",
            "status": "completed",
            "content": [{"type": "output_text", "text": "", "annotations": []}],
        },
    });
    let part = json!({
        "type": "response.content_part.done",
        "item_id": format!("msg_{id}"),
        "output_index": 0,
        "content_index": 0,
        "part": {"type": "output_text", "text": "", "annotations": []},
    });
    Frame::data(Bytes::from(format!(
        "data: {item}\n\ndata: {part}\n\n"
    )))
}

fn create_responses_tool_calls_frame(
    id: &str,
    model: &str,
    created: i64,
    tool_calls: &[ToolCall],
) -> Frame<Bytes> {
    let chunks = tool_calls
        .iter()
        .map(|call| {
            let added = json!({
                "type": "response.output_item.added",
                "item": {
                    "type": "function_call",
                    "id": format!("fc_{id}_{}", call.id.clone().unwrap_or_default()),
                    "call_id": call.id,
                    "name": call.name,
                    "arguments": call.arguments.to_string(),
                    "status": "completed",
                },
            });
            let done = json!({
                "type": "response.function_call_arguments.done",
                "item_id": format!("fc_{id}_{}", call.id.clone().unwrap_or_default()),
                "output_index": 0,
                "arguments": call.arguments.to_string(),
            });
            vec![
                format!("data: {added}\n\n"),
                format!("data: {done}\n\n"),
            ]
        })
        .collect::<Vec<_>>()
        .concat()
        .join("");
    let _ = (model, created);
    Frame::data(Bytes::from(chunks))
}

fn create_responses_error_frame(id: &str, model: &str, created: i64, err: &str) -> Frame<Bytes> {
    let mut value = build_response_object_json(id, model, created, "failed");
    value["error"] = json!({ "code": "server_error", "message": err });
    let event = json!({
        "type": "response.failed",
        "response": value,
    });
    Frame::data(Bytes::from(format!("data: {event}\n\n")))
}

fn create_responses_completed_frame(id: &str, model: &str, created: i64) -> Frame<Bytes> {
    let value = build_response_object_json(id, model, created, "completed");
    let event = json!({
        "type": "response.completed",
        "response": value,
    });
    Frame::data(Bytes::from(format!("data: {event}\n\n")))
}

/// Convert a ChatCompletionsOutput into a Responses API `Response` object.
fn ret_responses_non_stream(
    id: &str,
    model: &str,
    created: i64,
    output: &ChatCompletionsOutput,
) -> Bytes {
    let response_id = output.id.as_deref().unwrap_or(id);
    let mut output_items = vec![];
    if !output.text.is_empty() {
        output_items.push(json!({
            "type": "message",
            "id": format!("msg_{response_id}"),
            "role": "assistant",
            "status": "completed",
            "content": [
                {
                    "type": "output_text",
                    "text": output.text,
                    "annotations": [],
                }
            ],
        }));
    }
    for call in &output.tool_calls {
        output_items.push(json!({
            "type": "function_call",
            "id": format!("fc_{response_id}_{}", call.id.clone().unwrap_or_default()),
            "call_id": call.id,
            "name": call.name,
            "arguments": call.arguments.to_string(),
            "status": "completed",
        }));
    }
    let input_tokens = output.input_tokens.unwrap_or_default();
    let output_tokens = output.output_tokens.unwrap_or_default();
    let res_body = json!({
        "id": response_id,
        "object": "response",
        "created_at": created,
        "status": "completed",
        "model": model,
        "output": output_items,
        "usage": {
            "input_tokens": input_tokens,
            "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
            "output_tokens": output_tokens,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": input_tokens + output_tokens,
        },
        "error": null,
        "incomplete_details": null,
    });
    Bytes::from(res_body.to_string())
}

fn ret_non_stream(id: &str, model: &str, created: i64, output: &ChatCompletionsOutput) -> Bytes {
    let id = output.id.as_deref().unwrap_or(id);
    let input_tokens = output.input_tokens.unwrap_or_default();
    let output_tokens = output.output_tokens.unwrap_or_default();
    let total_tokens = input_tokens + output_tokens;
    let choice = if output.tool_calls.is_empty() {
        json!({
            "index": 0,
            "message": {
                "role": "assistant",
                "content": output.text,
            },
            "logprobs": null,
            "finish_reason": "stop",
        })
    } else {
        let content = if output.text.is_empty() {
            Value::Null
        } else {
            output.text.clone().into()
        };
        let tool_calls: Vec<_> = output
            .tool_calls
            .iter()
            .map(|call| {
                json!({
                    "id": call.id,
                    "type": "function",
                    "function": {
                        "name": call.name,
                        "arguments": call.arguments.to_string(),
                    }
                })
            })
            .collect();
        json!({
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
                "tool_calls": tool_calls,
            },
            "logprobs": null,
            "finish_reason": "tool_calls",
        })
    };
    let res_body = json!({
        "id": id,
        "object": "chat.completion",
        "created": created,
        "model": model,
        "choices": [choice],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": total_tokens,
        },
    });
    Bytes::from(res_body.to_string())
}

fn ret_err<T: std::fmt::Display>(err: T) -> AppResponse {
    let data = json!({
        "error": {
            "message": err.to_string(),
            "type": "invalid_request_error",
        },
    });
    Response::builder()
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(data.to_string())).boxed())
        .unwrap()
}

fn parse_messages(message: Vec<Value>) -> Result<Vec<Message>> {
    let mut output = vec![];
    let mut tool_results = None;
    for (i, message) in message.into_iter().enumerate() {
        let err = || anyhow!("Failed to parse '.messages[{i}]'");
        let role = message["role"].as_str().ok_or_else(err)?;
        let content = match message.get("content") {
            Some(value) => {
                if let Some(value) = value.as_str() {
                    MessageContent::Text(value.to_string())
                } else if value.is_array() {
                    let value = serde_json::from_value(value.clone()).map_err(|_| err())?;
                    MessageContent::Array(value)
                } else if value.is_null() {
                    MessageContent::Text(String::new())
                } else {
                    return Err(err());
                }
            }
            None => MessageContent::Text(String::new()),
        };
        match role {
            "system" | "user" => {
                let role = match role {
                    "system" => MessageRole::System,
                    "user" => MessageRole::User,
                    _ => unreachable!(),
                };
                output.push(Message::new(role, content))
            }
            "assistant" => {
                let role = MessageRole::Assistant;
                match message["tool_calls"].as_array() {
                    Some(tool_calls) => {
                        if tool_results.is_some() {
                            return Err(err());
                        }
                        let mut list = vec![];
                        for tool_call in tool_calls {
                            if let (id, Some(name), Some(arguments)) = (
                                tool_call["id"].as_str().map(|v| v.to_string()),
                                tool_call["function"]["name"].as_str(),
                                tool_call["function"]["arguments"].as_str(),
                            ) {
                                let arguments =
                                    serde_json::from_str(arguments).map_err(|_| err())?;
                                list.push((id, name.to_string(), arguments));
                            } else {
                                return Err(err());
                            }
                        }
                        tool_results = Some((content.to_text(), list, vec![]));
                    }
                    None => output.push(Message::new(role, content)),
                }
            }
            "tool" => match tool_results.take() {
                Some((text, tool_calls, mut tool_values)) => {
                    let tool_call_id = message["tool_call_id"].as_str().map(|v| v.to_string());
                    let content = content.to_text();
                    let value: Value = serde_json::from_str(&content)
                        .ok()
                        .unwrap_or_else(|| content.into());

                    tool_values.push((value, tool_call_id));

                    if tool_calls.len() == tool_values.len() {
                        let mut list = vec![];
                        for ((id, name, arguments), (value, tool_call_id)) in
                            tool_calls.into_iter().zip(tool_values.into_iter())
                        {
                            if id != tool_call_id {
                                return Err(err());
                            }
                            list.push(ToolResult::new(ToolCall::new(name, arguments, id), value))
                        }
                        output.push(Message::new(
                            MessageRole::Assistant,
                            MessageContent::ToolCalls(MessageContentToolCalls::new(list, text)),
                        ));
                        tool_results = None;
                    } else {
                        tool_results = Some((text, tool_calls, tool_values));
                    }
                }
                None => return Err(err()),
            },
            _ => {
                return Err(err());
            }
        }
    }

    if tool_results.is_some() {
        bail!("Invalid messages");
    }

    Ok(output)
}

fn parse_tools(tools: Option<Vec<Value>>) -> Result<Option<Vec<FunctionDeclaration>>> {
    let tools = match tools {
        Some(v) => v,
        None => return Ok(None),
    };
    let mut functions = vec![];
    for (i, tool) in tools.into_iter().enumerate() {
        if let (Some("function"), Some(function)) = (
            tool["type"].as_str(),
            tool["function"]
                .as_object()
                .and_then(|v| serde_json::from_value(json!(v)).ok()),
        ) {
            functions.push(function);
        } else {
            bail!("Failed to parse '.tools[{i}]'")
        }
    }
    Ok(Some(functions))
}
