pub mod error;
pub mod types;

pub use error::{PacError, Result};
pub use types::*;

/// 系統版本與專案常數
pub const PAC_ROUTE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_ROUTER_PORT: u16 = 8088;
pub const DEFAULT_MDNS_SERVICE_TYPE: &str = "_pacroute._tcp.local.";
