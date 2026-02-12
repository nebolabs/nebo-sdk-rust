use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// A complete renderable UI view.
#[derive(Debug, Clone)]
pub struct View {
    pub view_id: String,
    pub title: String,
    pub blocks: Vec<UiBlock>,
}

/// A single renderable UI element.
#[derive(Debug, Clone)]
pub struct UiBlock {
    pub block_id: String,
    pub r#type: String,
    pub text: String,
    pub value: String,
    pub placeholder: String,
    pub hint: String,
    pub variant: String,
    pub src: String,
    pub alt: String,
    pub disabled: bool,
    pub options: Vec<SelectOption>,
    pub style: String,
}

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

/// A user interaction event.
#[derive(Debug, Clone)]
pub struct UiEvent {
    pub view_id: String,
    pub block_id: String,
    pub action: String,
    pub value: String,
}

/// Response to a UI event.
#[derive(Debug, Clone)]
pub struct UiEventResult {
    pub view: Option<View>,
    pub error: String,
    pub toast: String,
}

/// Trait for UI capability handlers.
#[async_trait]
pub trait UiHandler: Send + Sync + 'static {
    async fn get_view(&self, context: &str) -> Result<View, NeboError>;
    async fn on_event(&self, event: UiEvent) -> Result<UiEventResult, NeboError>;
}

/// Optional extension for streaming UI updates.
#[async_trait]
pub trait UiHandlerWithStreaming: UiHandler {
    async fn stream_updates(&self) -> Result<mpsc::Receiver<View>, NeboError>;
}

pub(crate) struct UiBridge {
    pub handler: Box<dyn UiHandler>,
    pub on_configure: Option<Box<dyn Fn(std::collections::HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
}

fn view_to_proto(v: &View) -> pb::UiView {
    pb::UiView {
        view_id: v.view_id.clone(),
        title: v.title.clone(),
        blocks: v
            .blocks
            .iter()
            .map(|b| pb::UiBlock {
                block_id: b.block_id.clone(),
                r#type: b.r#type.clone(),
                text: b.text.clone(),
                value: b.value.clone(),
                placeholder: b.placeholder.clone(),
                hint: b.hint.clone(),
                variant: b.variant.clone(),
                src: b.src.clone(),
                alt: b.alt.clone(),
                disabled: b.disabled,
                options: b
                    .options
                    .iter()
                    .map(|o| pb::SelectOption {
                        label: o.label.clone(),
                        value: o.value.clone(),
                    })
                    .collect(),
                style: b.style.clone(),
            })
            .collect(),
    }
}

#[tonic::async_trait]
impl pb::ui_service_server::UiService for UiBridge {
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

    async fn get_view(
        &self,
        req: Request<pb::GetViewRequest>,
    ) -> Result<Response<pb::UiView>, Status> {
        let view = self
            .handler
            .get_view(&req.into_inner().context)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(view_to_proto(&view)))
    }

    async fn send_event(
        &self,
        req: Request<pb::UiEvent>,
    ) -> Result<Response<pb::UiEventResponse>, Status> {
        let inner = req.into_inner();
        let result = self
            .handler
            .on_event(UiEvent {
                view_id: inner.view_id,
                block_id: inner.block_id,
                action: inner.action,
                value: inner.value,
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(pb::UiEventResponse {
            view: result.view.as_ref().map(view_to_proto),
            error: result.error,
            toast: result.toast,
        }))
    }

    type StreamUpdatesStream =
        tokio_stream::wrappers::ReceiverStream<Result<pb::UiView, Status>>;

    async fn stream_updates(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<Self::StreamUpdatesStream>, Status> {
        // Default: keep stream open but don't send anything
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
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
