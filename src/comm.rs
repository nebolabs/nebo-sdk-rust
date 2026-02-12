use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// An inter-agent communication message.
#[derive(Debug, Clone)]
pub struct CommMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub topic: String,
    pub conversation_id: String,
    pub r#type: String,
    pub content: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub timestamp: i64,
    pub human_injected: bool,
    pub human_id: String,
}

/// Trait for comm capability handlers.
#[async_trait]
pub trait CommHandler: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn connect(&self, config: std::collections::HashMap<String, String>) -> Result<(), NeboError>;
    async fn disconnect(&self) -> Result<(), NeboError>;
    fn is_connected(&self) -> bool;
    async fn send(&self, msg: CommMessage) -> Result<(), NeboError>;
    async fn subscribe(&self, topic: &str) -> Result<(), NeboError>;
    async fn unsubscribe(&self, topic: &str) -> Result<(), NeboError>;
    async fn register(&self, agent_id: &str, capabilities: &[String]) -> Result<(), NeboError>;
    async fn deregister(&self) -> Result<(), NeboError>;
    async fn receive(&self) -> Result<mpsc::Receiver<CommMessage>, NeboError>;
}

pub(crate) struct CommBridge {
    pub handler: Box<dyn CommHandler>,
    pub on_configure: Option<Box<dyn Fn(std::collections::HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
}

fn to_proto_comm(m: &CommMessage) -> pb::CommMessage {
    pb::CommMessage {
        id: m.id.clone(),
        from: m.from.clone(),
        to: m.to.clone(),
        topic: m.topic.clone(),
        conversation_id: m.conversation_id.clone(),
        r#type: m.r#type.clone(),
        content: m.content.clone(),
        metadata: m.metadata.clone(),
        timestamp: m.timestamp,
        human_injected: m.human_injected,
        human_id: m.human_id.clone(),
    }
}

fn from_proto_comm(m: &pb::CommMessage) -> CommMessage {
    CommMessage {
        id: m.id.clone(),
        from: m.from.clone(),
        to: m.to.clone(),
        topic: m.topic.clone(),
        conversation_id: m.conversation_id.clone(),
        r#type: m.r#type.clone(),
        content: m.content.clone(),
        metadata: m.metadata.clone(),
        timestamp: m.timestamp,
        human_injected: m.human_injected,
        human_id: m.human_id.clone(),
    }
}

#[tonic::async_trait]
impl pb::comm_service_server::CommService for CommBridge {
    async fn health_check(&self, _req: Request<pb::HealthCheckRequest>) -> Result<Response<pb::HealthCheckResponse>, Status> {
        Ok(Response::new(pb::HealthCheckResponse { healthy: true, name: self.env.name.clone(), version: self.env.version.clone() }))
    }

    async fn name(&self, _req: Request<pb::Empty>) -> Result<Response<pb::CommNameResponse>, Status> {
        Ok(Response::new(pb::CommNameResponse { name: self.handler.name().to_string() }))
    }

    async fn version(&self, _req: Request<pb::Empty>) -> Result<Response<pb::CommVersionResponse>, Status> {
        Ok(Response::new(pb::CommVersionResponse { version: self.handler.version().to_string() }))
    }

    async fn connect(&self, req: Request<pb::CommConnectRequest>) -> Result<Response<pb::CommConnectResponse>, Status> {
        match self.handler.connect(req.into_inner().config).await {
            Ok(()) => Ok(Response::new(pb::CommConnectResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommConnectResponse { error: e.to_string() })),
        }
    }

    async fn disconnect(&self, _req: Request<pb::Empty>) -> Result<Response<pb::CommDisconnectResponse>, Status> {
        match self.handler.disconnect().await {
            Ok(()) => Ok(Response::new(pb::CommDisconnectResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommDisconnectResponse { error: e.to_string() })),
        }
    }

    async fn is_connected(&self, _req: Request<pb::Empty>) -> Result<Response<pb::CommIsConnectedResponse>, Status> {
        Ok(Response::new(pb::CommIsConnectedResponse { connected: self.handler.is_connected() }))
    }

    async fn send(&self, req: Request<pb::CommSendRequest>) -> Result<Response<pb::CommSendResponse>, Status> {
        let msg = req.into_inner().message.map(|m| from_proto_comm(&m)).unwrap_or(CommMessage {
            id: String::new(), from: String::new(), to: String::new(), topic: String::new(),
            conversation_id: String::new(), r#type: String::new(), content: String::new(),
            metadata: std::collections::HashMap::new(), timestamp: 0, human_injected: false, human_id: String::new(),
        });
        match self.handler.send(msg).await {
            Ok(()) => Ok(Response::new(pb::CommSendResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommSendResponse { error: e.to_string() })),
        }
    }

    async fn subscribe(&self, req: Request<pb::CommSubscribeRequest>) -> Result<Response<pb::CommSubscribeResponse>, Status> {
        match self.handler.subscribe(&req.into_inner().topic).await {
            Ok(()) => Ok(Response::new(pb::CommSubscribeResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommSubscribeResponse { error: e.to_string() })),
        }
    }

    async fn unsubscribe(&self, req: Request<pb::CommUnsubscribeRequest>) -> Result<Response<pb::CommUnsubscribeResponse>, Status> {
        match self.handler.unsubscribe(&req.into_inner().topic).await {
            Ok(()) => Ok(Response::new(pb::CommUnsubscribeResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommUnsubscribeResponse { error: e.to_string() })),
        }
    }

    async fn register(&self, req: Request<pb::CommRegisterRequest>) -> Result<Response<pb::CommRegisterResponse>, Status> {
        let inner = req.into_inner();
        match self.handler.register(&inner.agent_id, &inner.capabilities).await {
            Ok(()) => Ok(Response::new(pb::CommRegisterResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommRegisterResponse { error: e.to_string() })),
        }
    }

    async fn deregister(&self, _req: Request<pb::Empty>) -> Result<Response<pb::CommDeregisterResponse>, Status> {
        match self.handler.deregister().await {
            Ok(()) => Ok(Response::new(pb::CommDeregisterResponse { error: String::new() })),
            Err(e) => Ok(Response::new(pb::CommDeregisterResponse { error: e.to_string() })),
        }
    }

    type ReceiveStream = tokio_stream::wrappers::ReceiverStream<Result<pb::CommMessage, Status>>;

    async fn receive(&self, _req: Request<pb::Empty>) -> Result<Response<Self::ReceiveStream>, Status> {
        let mut rx = self.handler.receive().await.map_err(|e| Status::internal(e.to_string()))?;
        let (tx, stream_rx) = mpsc::channel(100);
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if tx.send(Ok(to_proto_comm(&msg))).await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(stream_rx)))
    }

    async fn configure(&self, req: Request<pb::SettingsMap>) -> Result<Response<pb::Empty>, Status> {
        if let Some(ref cb) = self.on_configure {
            cb(req.into_inner().values);
        }
        Ok(Response::new(pb::Empty {}))
    }
}
