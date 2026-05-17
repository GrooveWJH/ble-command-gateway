pub mod ble;
pub mod discovery;
pub mod request;
pub mod response;
pub mod scan_state;
pub mod session;

#[cfg(test)]
mod ble_tests;

// Shared client library exports
pub use ble::{
    sort_scan_candidates, BleClient, ScanCandidateInfo, ScanProgressEvent, ScanRunSummary,
    ScannedDevice,
};
pub use discovery::{DiscoveryCriteria, SHORT_NAME_PREFIX, UART_SERVICE_UUID};
pub use request::{build_request, encode_request_bytes, prepare_request, PreparedRequest};
pub use session::BleSession;
