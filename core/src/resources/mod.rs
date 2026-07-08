//! API resource operations: high-level commands that turn typed parameters into
//! HTTP requests and return output-ready JSON. Ported 1:1 from the predecessor
//! Python tool. Extended per resource in later tasks.

pub mod api;
pub mod attachment;
pub mod comment;
pub mod notification;
pub mod relation;
pub mod time;
pub mod work_packages;
