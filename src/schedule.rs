use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::env::AppEnv;
use crate::error::NeboError;
use crate::pb;

/// Re-export protobuf schedule types for handler implementations.
pub use pb::Schedule;
pub use pb::ScheduleTrigger;
pub use pb::ScheduleHistoryEntry;
pub use pb::CreateScheduleRequest;
pub use pb::UpdateScheduleRequest;

/// Trait for schedule capability handlers.
#[async_trait]
pub trait ScheduleHandler: Send + Sync + 'static {
    async fn create(&self, req: CreateScheduleRequest) -> Result<Schedule, NeboError>;
    async fn get(&self, name: &str) -> Result<Schedule, NeboError>;
    async fn list(&self, limit: i32, offset: i32, enabled_only: bool) -> Result<(Vec<Schedule>, i64), NeboError>;
    async fn update(&self, req: UpdateScheduleRequest) -> Result<Schedule, NeboError>;
    async fn delete(&self, name: &str) -> Result<(), NeboError>;
    async fn enable(&self, name: &str) -> Result<Schedule, NeboError>;
    async fn disable(&self, name: &str) -> Result<Schedule, NeboError>;
    async fn trigger(&self, name: &str) -> Result<(bool, String), NeboError>;
    async fn history(&self, name: &str, limit: i32, offset: i32) -> Result<(Vec<ScheduleHistoryEntry>, i64), NeboError>;
    async fn triggers(&self) -> Result<mpsc::Receiver<ScheduleTrigger>, NeboError>;
}

pub(crate) struct ScheduleBridge {
    pub handler: Box<dyn ScheduleHandler>,
    pub on_configure: Option<Box<dyn Fn(std::collections::HashMap<String, String>) + Send + Sync>>,
    pub env: AppEnv,
}

#[tonic::async_trait]
impl pb::schedule_service_server::ScheduleService for ScheduleBridge {
    async fn health_check(&self, _req: Request<pb::HealthCheckRequest>) -> Result<Response<pb::HealthCheckResponse>, Status> {
        Ok(Response::new(pb::HealthCheckResponse { healthy: true, name: self.env.name.clone(), version: self.env.version.clone() }))
    }

    async fn create(&self, req: Request<pb::CreateScheduleRequest>) -> Result<Response<pb::ScheduleResponse>, Status> {
        match self.handler.create(req.into_inner()).await {
            Ok(s) => Ok(Response::new(pb::ScheduleResponse { schedule: Some(s), error: String::new() })),
            Err(e) => Ok(Response::new(pb::ScheduleResponse { schedule: None, error: e.to_string() })),
        }
    }

    async fn get(&self, req: Request<pb::GetScheduleRequest>) -> Result<Response<pb::ScheduleResponse>, Status> {
        match self.handler.get(&req.into_inner().name).await {
            Ok(s) => Ok(Response::new(pb::ScheduleResponse { schedule: Some(s), error: String::new() })),
            Err(e) => Ok(Response::new(pb::ScheduleResponse { schedule: None, error: e.to_string() })),
        }
    }

    async fn list(&self, req: Request<pb::ListSchedulesRequest>) -> Result<Response<pb::ListSchedulesResponse>, Status> {
        let inner = req.into_inner();
        match self.handler.list(inner.limit, inner.offset, inner.enabled_only).await {
            Ok((schedules, total)) => Ok(Response::new(pb::ListSchedulesResponse { schedules, total })),
            Err(_) => Ok(Response::new(pb::ListSchedulesResponse { schedules: vec![], total: 0 })),
        }
    }

    async fn update(&self, req: Request<pb::UpdateScheduleRequest>) -> Result<Response<pb::ScheduleResponse>, Status> {
        match self.handler.update(req.into_inner()).await {
            Ok(s) => Ok(Response::new(pb::ScheduleResponse { schedule: Some(s), error: String::new() })),
            Err(e) => Ok(Response::new(pb::ScheduleResponse { schedule: None, error: e.to_string() })),
        }
    }

    async fn delete(&self, req: Request<pb::DeleteScheduleRequest>) -> Result<Response<pb::DeleteScheduleResponse>, Status> {
        match self.handler.delete(&req.into_inner().name).await {
            Ok(()) => Ok(Response::new(pb::DeleteScheduleResponse { success: true, error: String::new() })),
            Err(e) => Ok(Response::new(pb::DeleteScheduleResponse { success: false, error: e.to_string() })),
        }
    }

    async fn enable(&self, req: Request<pb::ScheduleNameRequest>) -> Result<Response<pb::ScheduleResponse>, Status> {
        match self.handler.enable(&req.into_inner().name).await {
            Ok(s) => Ok(Response::new(pb::ScheduleResponse { schedule: Some(s), error: String::new() })),
            Err(e) => Ok(Response::new(pb::ScheduleResponse { schedule: None, error: e.to_string() })),
        }
    }

    async fn disable(&self, req: Request<pb::ScheduleNameRequest>) -> Result<Response<pb::ScheduleResponse>, Status> {
        match self.handler.disable(&req.into_inner().name).await {
            Ok(s) => Ok(Response::new(pb::ScheduleResponse { schedule: Some(s), error: String::new() })),
            Err(e) => Ok(Response::new(pb::ScheduleResponse { schedule: None, error: e.to_string() })),
        }
    }

    async fn trigger(&self, req: Request<pb::ScheduleNameRequest>) -> Result<Response<pb::TriggerResponse>, Status> {
        match self.handler.trigger(&req.into_inner().name).await {
            Ok((success, output)) => Ok(Response::new(pb::TriggerResponse { success, output, error: String::new() })),
            Err(e) => Ok(Response::new(pb::TriggerResponse { success: false, output: String::new(), error: e.to_string() })),
        }
    }

    async fn history(&self, req: Request<pb::ScheduleHistoryRequest>) -> Result<Response<pb::ScheduleHistoryResponse>, Status> {
        let inner = req.into_inner();
        match self.handler.history(&inner.name, inner.limit, inner.offset).await {
            Ok((entries, total)) => Ok(Response::new(pb::ScheduleHistoryResponse { entries, total })),
            Err(_) => Ok(Response::new(pb::ScheduleHistoryResponse { entries: vec![], total: 0 })),
        }
    }

    type TriggersStream = tokio_stream::wrappers::ReceiverStream<Result<pb::ScheduleTrigger, Status>>;

    async fn triggers(&self, _req: Request<pb::Empty>) -> Result<Response<Self::TriggersStream>, Status> {
        let mut rx = self.handler.triggers().await.map_err(|e| Status::internal(e.to_string()))?;
        let (tx, stream_rx) = mpsc::channel(100);
        tokio::spawn(async move {
            while let Some(trigger) = rx.recv().await {
                if tx.send(Ok(trigger)).await.is_err() {
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
