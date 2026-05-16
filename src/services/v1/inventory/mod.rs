//! Business logic for inventory and product management.
//!
//! This module contains the 'decide' functions that encapsulate the core rules
//! for part registration, product listing, and the approval/denial workflow.

pub mod add_parts;
pub mod list_parts;
pub mod get_part;
pub mod list_products;
pub mod get_product;
pub mod check_serial;
pub mod register_product;
pub mod accept_part;
pub mod deny_part;
