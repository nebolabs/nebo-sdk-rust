use async_trait::async_trait;
use serde_json::Value;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// Trait for tool capability handlers.
#[async_trait]
pub trait ToolHandler: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> Value;
    async fn execute(&self, input: Value) -> Result<String, NeboError>;
    fn requires_approval(&self) -> bool {
        false
    }
}

pub(crate) struct ToolBridge {
    pub handler: Box<dyn ToolHandler>,
    pub on_configure: Option<Box<dyn Fn(std::collections::HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
}

#[tonic::async_trait]
impl pb::tool_service_server::ToolService for ToolBridge {
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

    async fn name(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<pb::NameResponse>, Status> {
        Ok(Response::new(pb::NameResponse {
            name: self.handler.name().to_string(),
        }))
    }

    async fn description(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<pb::DescriptionResponse>, Status> {
        Ok(Response::new(pb::DescriptionResponse {
            description: self.handler.description().to_string(),
        }))
    }

    async fn schema(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<pb::SchemaResponse>, Status> {
        let schema = serde_json::to_vec(&self.handler.schema()).unwrap_or_default();
        Ok(Response::new(pb::SchemaResponse { schema }))
    }

    async fn execute(
        &self,
        req: Request<pb::ExecuteRequest>,
    ) -> Result<Response<pb::ExecuteResponse>, Status> {
        let input: Value = serde_json::from_slice(&req.into_inner().input)
            .unwrap_or(Value::Object(serde_json::Map::new()));

        match self.handler.execute(input).await {
            Ok(content) => Ok(Response::new(pb::ExecuteResponse {
                content,
                is_error: false,
            })),
            Err(e) => Ok(Response::new(pb::ExecuteResponse {
                content: e.to_string(),
                is_error: true,
            })),
        }
    }

    async fn requires_approval(
        &self,
        _req: Request<pb::Empty>,
    ) -> Result<Response<pb::ApprovalResponse>, Status> {
        Ok(Response::new(pb::ApprovalResponse {
            requires_approval: self.handler.requires_approval(),
        }))
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
