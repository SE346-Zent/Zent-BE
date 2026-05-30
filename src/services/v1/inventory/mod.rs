//! Business logic for inventory and product management.
//!
//! This module contains the 'decide' functions that encapsulate the core rules
//! for part registration, product management, and the approval/denial workflow.

pub mod add_parts;
pub mod get_product;
pub mod register_device;
pub mod accept_part;
pub mod deny_part;
pub mod ports;
pub mod check_warranty;
pub mod analytics;
pub mod new_part_forms;
