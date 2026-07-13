//! Stable SDK-originated error codes.
//!
//! Provider business codes occupy `1000..=1499`; SDK failures use the
//! reserved `9000..=9999` band so callers can distinguish their origin.

/// Generic client-side validation failure (bad argument shape, …).
pub const SDK_VALIDATION: u16 = 9001;

/// Client-side configuration error (bad base URL, missing value, …).
pub const SDK_CONFIG: u16 = 9600;

/// A local file referenced by the request does not exist.
pub const SDK_FILE_NOT_FOUND: u16 = 9100;

/// A local file exceeds the SDK-enforced size limit.
pub const SDK_FILE_TOO_LARGE: u16 = 9101;

/// The file type or extension is unsupported by the target operation.
pub const SDK_FILE_TYPE_UNSUPPORTED: u16 = 9102;

/// Generic local I/O failure (read, write, or permission failure).
pub const SDK_IO: u16 = 9400;

/// A client-side timeout, such as exhausting a polling deadline.
pub const SDK_TIMEOUT: u16 = 9300;

/// A failure reported by an external tool or toolkit integration.
pub const SDK_EXTERNAL_TOOL: u16 = 9500;
