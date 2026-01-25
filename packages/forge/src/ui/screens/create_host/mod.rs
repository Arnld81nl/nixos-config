//! Create host wizard screens
//!
//! This module contains all UI screens for the host creation wizard:
//! - Hardware detection and confirmation (CPU, GPU, form factor)
//! - Disk selection and hostname entry
//! - Configuration review and generation progress

mod disk;
mod generation;
mod hardware;
mod helpers;

// Re-export all public draw functions for external use
pub use disk::{draw_enter_hostname, draw_select_disk};
pub use generation::{draw_complete, draw_generating, draw_review};
pub use hardware::{draw_confirm_cpu, draw_confirm_form_factor, draw_confirm_gpu, draw_detecting_hardware};
