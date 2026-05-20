//! Shared Core Services (v1)
//!
//! This module provides cross-cutting domain logic used by multiple services,
//! including email dispatch, token generation/validation, and media processing.

pub mod email_service;
pub mod token_service;
pub mod media;
pub mod helpers;
