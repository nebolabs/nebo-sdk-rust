use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// An LLM chat completion request from Nebo.
#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub request_id: String,
    pub messages: Vec<GatewayMessage>,
    pub tools: Vec<GatewayToolDef>,
    pub max_tokens: i32,
    pub temperature: f64,
    pub system: String,
    pub user_id: String,
    pub user_plan: String,
    pub user_token: String,
}

#[derive(Debug, Clone)]
pub struct GatewayMessage {
    pub role: String,
    pub content: String,
    pub tool_call_id: String,
    pub tool_calls: String,
}

#[derive(Debug, Clone)]
pub struct GatewayToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Vec<u8>,
}

/// A streamed event sent back to Nebo.
#[derive(Debug, Clone)]
pub struct GatewayEvent {
    pub r#type: String,
    pub content: String,
    pub model: String,
    pub request_id: String,
}

/// Trait for gateway capability handlers.
#[async_trait]
pub trait GatewayHandler: Send + Sync + 'static {
    async fn stream(&self, req: GatewayRequest) -> Result<mpsc::Receiver<GatewayEvent>, NeboError>;
    async fn cancel(&self, request_id: &str) -> Result<(), NeboError>;
}

pub(crate) struct GatewayBridge {
    pub handler: Box<dyn GatewayHandler>,
    pub on_configure: Option<Box<dyn Fn(std::collections::HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
}

#[tonic::async_trait]
impl pb::gateway_service_server::GatewayService for GatewayBridge {
    async fn health_check(
        &self,
        _req: Request<pb::HealthCheckRequest>,
    ) -> Result<Response<pb::HealthCheckResponse>, Status> {
        Ok(Response::new(pb::HealthCheckResponse {
            healthy: true,
            name: self.env.name.clone(),
            version: self.env.version.clone(),
        }))
    }

    type StreamStream = tokio_stream::wrappers::ReceiverStream<Result<pb::GatewayEvent, Status>>;

    async fn stream(
        &self,
        req: Request<pb::GatewayRequest>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let inner = req.into_inner();
        let gw_req = GatewayRequest {
            request_id: inner.request_id.clone(),
            max_tokens: inner.max_tokens,
            temperature: inner.temperature,
            system: inner.system,
            user_id: inner.user.as_ref().map(|u| u.user_id.clone()).unwrap_or_default(),
            user_plan: inner.user.as_ref().map(|u| u.plan.clone()).unwrap_or_default(),
            user_token: inner.user.as_ref().map(|u| u.token.clone()).unwrap_or_default(),
            messages: inner.messages.into_iter().map(|m| GatewayMessage {
                role: m.role,
                content: m.content,
                tool_call_id: m.tool_call_id,
                tool_calls: m.tool_calls,
            }).collect(),
            tools: inner.tools.into_iter().map(|t| GatewayToolDef {
                name: t.name,
                description: t.description,
                input_schema: t.input_schema,
            }).collect(),
        };

        let mut rx = self
            .handler
            .stream(gw_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (tx, stream_rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let proto = pb::GatewayEvent {
                    r#type: event.r#type,
                    content: event.content,
                    model: event.model,
                    request_id: event.request_id,
                };
                if tx.send(Ok(proto)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(stream_rx)))
    }

    async fn poll(
        &self,
        _req: Request<pb::PollRequest>,
    ) -> Result<Response<pb::PollResponse>, Status> {
        Ok(Response::new(pb::PollResponse {
            events: vec![],
            complete: false,
        }))
    }

    async fn cancel(
        &self,
        req: Request<pb::CancelRequest>,
    ) -> Result<Response<pb::CancelResponse>, Status> {
        match self.handler.cancel(&req.into_inner().request_id).await {
            Ok(()) => Ok(Response::new(pb::CancelResponse { cancelled: true })),
            Err(_) => Ok(Response::new(pb::CancelResponse { cancelled: false })),
        }
    }

    async fn configure(
        &self,
        req: Request<pb::SettingsMap>,
    ) -> Result<Response<pb::Empty>, Status> {
        if let Some(ref cb) = self.on_configure {
            cb(req.into_inner().values);
        }
        Ok(Response::new(pb::Empty {}))
    }
}
