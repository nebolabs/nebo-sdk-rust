use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// Identifies who sent a message.
#[derive(Debug, Clone, Default)]
pub struct MessageSender {
    pub name: String,
    pub role: String,
    pub bot_id: String,
}

/// A file or media attachment.
#[derive(Debug, Clone, Default)]
pub struct Attachment {
    pub r#type: String,
    pub url: String,
    pub filename: String,
    pub size: i64,
}

/// An interactive element (button, keyboard row).
#[derive(Debug, Clone, Default)]
pub struct MessageAction {
    pub label: String,
    pub callback_id: String,
}

/// Channel message envelope used for both sending and receiving.
#[derive(Debug, Clone, Default)]
pub struct ChannelEnvelope {
    pub message_id: String,
    pub channel_id: String,
    pub sender: MessageSender,
    pub text: String,
    pub attachments: Vec<Attachment>,
    pub reply_to: String,
    pub actions: Vec<MessageAction>,
    pub platform_data: Vec<u8>,
    pub timestamp: String,
    // Legacy fields
    pub user_id: String,
    pub metadata: String,
}

/// Trait for channel capability handlers.
#[async_trait]
pub trait ChannelHandler: Send + Sync + 'static {
    fn id(&self) -> &str;
    async fn connect(&self, config: std::collections::HashMap<String, String>) -> Result<(), NeboError>;
    async fn disconnect(&self) -> Result<(), NeboError>;
    /// Send a message. Returns the platform-assigned message ID.
    async fn send(&self, env: ChannelEnvelope) -> Result<String, NeboError>;
    async fn receive(&self) -> Result<mpsc::Receiver<ChannelEnvelope>, NeboError>;
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
        let sender = inner.sender.map(|s| MessageSender {
            name: s.name,
            role: s.role,
            bot_id: s.bot_id,
        }).unwrap_or_default();

        let env = ChannelEnvelope {
            message_id: inner.message_id,
            channel_id: inner.channel_id,
            sender,
            text: inner.text,
            attachments: inner.attachments.into_iter().map(|a| Attachment {
                r#type: a.r#type,
                url: a.url,
                filename: a.filename,
                size: a.size,
            }).collect(),
            reply_to: inner.reply_to,
            actions: inner.actions.into_iter().map(|a| MessageAction {
                label: a.label,
                callback_id: a.callback_id,
            }).collect(),
            platform_data: inner.platform_data,
            ..Default::default()
        };

        match self.handler.send(env).await {
            Ok(message_id) => Ok(Response::new(pb::ChannelSendResponse {
                error: String::new(),
                message_id,
            })),
            Err(e) => Ok(Response::new(pb::ChannelSendResponse {
                error: e.to_string(),
                message_id: String::new(),
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
                    message_id: msg.message_id,
                    sender: Some(pb::MessageSender {
                        name: msg.sender.name,
                        role: msg.sender.role,
                        bot_id: msg.sender.bot_id,
                    }),
                    attachments: msg.attachments.into_iter().map(|a| pb::Attachment {
                        r#type: a.r#type,
                        url: a.url,
                        filename: a.filename,
                        size: a.size,
                    }).collect(),
                    reply_to: msg.reply_to,
                    actions: msg.actions.into_iter().map(|a| pb::MessageAction {
                        label: a.label,
                        callback_id: a.callback_id,
                    }).collect(),
                    platform_data: msg.platform_data,
                    timestamp: msg.timestamp,
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
