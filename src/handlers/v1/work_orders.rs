use axum::{extract::State, http::StatusCode};
use std::sync::Arc;
use crate::services::v1::work_orders::WorkOrderService;

pub async fn create(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn list(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn get_details(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn assign(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn schedule(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn start(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn refuse(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn cancel(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn complete(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn history(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn add_parts(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn approve_refusal(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
pub async fn deny_refusal(State(_service): State<Arc<WorkOrderService>>) -> StatusCode { unimplemented!() }
