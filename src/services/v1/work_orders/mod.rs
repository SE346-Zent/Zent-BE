//! Work Order Domain Services (v1)
//!
//! This module provides pure domain logic for managing the work order lifecycle,
//! including creation, assignment, status transitions (start, complete, cancel),
//! and specialized workflows like refusal handling and history tracking.

pub mod create;
pub mod list;
pub mod get_details;
pub mod assign;
pub mod auto_assign;
pub mod start;
pub mod reassign;
pub mod refuse;
pub mod cancel;
pub mod complaint;
pub mod complete;
pub mod escalation;
pub mod history;
pub mod approve_refusal;
pub mod deny_refusal;
pub mod change_appointment;
pub mod reject_forms;
