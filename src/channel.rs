use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// An inbound message from an external platform.
#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel_id: String,
    pub user_id: String,
    pub text: String,
    pub metadata: String,
}

/// Trait for channel capability handlers.
#[async_trait]
pub trait ChannelHandler: Send + Sync + 'static {
    fn id(&self) -> &str;
    async fn connect(&self, config: std::collections::HashMap<String, String>) -> Result<(), NeboError>;
    async fn disconnect(&self) -> Result<(), NeboError>;
    async fn send(&self, channel_id: &str, text: &str) -> Result<(), NeboError>;
    async fn receive(&self) -> Result<mpsc::Receiver<InboundMessage>, NeboError>;
}

pub(crate) struct ChannelBridge {
    pub handler: Box<dyn ChannelHandler>,
    pub on_configure: Option<Box<dyn Fn(std::collections::HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
}

#[tonic::async_trait]
impl pb::channel_service_server::ChannelService for ChannelBridge {
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

    async fn id(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<pb::IdResponse>, Status> {
        Ok(Response::new(pb::IdResponse {
            id: self.handler.id().to_string(),
        }))
    }

    async fn connect(
        &self,
        req: Request<pb::ChannelConnectRequest>,
    ) -> Result<Response<pb::ChannelConnectResponse>, Status> {
        match self.handler.connect(req.into_inner().config).await {
            Ok(()) => Ok(Response::new(pb::ChannelConnectResponse {
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(pb::ChannelConnectResponse {
                error: e.to_string(),
            })),
        }
    }

    async fn disconnect(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<pb::ChannelDisconnectResponse>, Status> {
        match self.handler.disconnect().await {
            Ok(()) => Ok(Response::new(pb::ChannelDisconnectResponse {
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(pb::ChannelDisconnectResponse {
                error: e.to_string(),
            })),
        }
    }

    async fn send(
        &self,
        req: Request<pb::ChannelSendRequest>,
    ) -> Result<Response<pb::ChannelSendResponse>, Status> {
        let inner = req.into_inner();
        match self.handler.send(&inner.channel_id, &inner.text).await {
            Ok(()) => Ok(Response::new(pb::ChannelSendResponse {
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(pb::ChannelSendResponse {
                error: e.to_string(),
            })),
        }
    }

    type ReceiveStream = tokio_stream::wrappers::ReceiverStream<Result<pb::InboundMessage, Status>>;

    async fn receive(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<Self::ReceiveStream>, Status> {
        let mut rx = self
            .handler
            .receive()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (tx, stream_rx) = mpsc::channel(100);
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let proto_msg = pb::InboundMessage {
                    channel_id: msg.channel_id,
                    user_id: msg.user_id,
                    text: msg.text,
                    metadata: msg.metadata,
                };
                if tx.send(Ok(proto_msg)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            stream_rx,
        )))
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
