mod create;
mod list;
mod get_details;
mod assign;
mod schedule;
mod start;
mod refuse;
mod cancel;
mod complete;
mod history;
mod add_parts;
mod approve_refusal;
mod deny_refusal;

#[derive(Clone)]
pub struct WorkOrderService;

impl WorkOrderService {
    pub fn new() -> Self {
        Self
    }
}
