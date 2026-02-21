use async_trait::async_trait;
use std::collections::HashMap;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// An HTTP request proxied from the browser to the app.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// An HTTP response from the app back to the browser.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: i32,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// Trait for UI capability handlers.
#[async_trait]
pub trait UiHandler: Send + Sync + 'static {
    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse, NeboError>;
}

pub(crate) struct UiBridge {
    pub handler: Box<dyn UiHandler>,
    pub on_configure: Option<Box<dyn Fn(HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
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

    async fn handle_request(
        &self,
        req: Request<pb::HttpRequest>,
    ) -> Result<Response<pb::HttpResponse>, Status> {
        let inner = req.into_inner();
        let http_req = HttpRequest {
            method: inner.method,
            path: inner.path,
            query: inner.query,
            headers: inner.headers,
            body: inner.body,
        };

        let result = self
            .handler
            .handle_request(http_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(pb::HttpResponse {
            status_code: result.status_code,
            headers: result.headers,
            body: result.body,
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
