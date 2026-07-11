//! Crate-private adapter bridging the new [`ZaiClient`] to the legacy 0.4
//! per-request types during the P02–P05 migration (plan P02.12).
//!
//! The legacy request types (`ChatCompletion`, `FileUploadRequest`, …) still
//! carry their own `key`/`url`/`EndpointConfig`/`HttpClientConfig` and dispatch
//! through the free functions in `client::http`. Until P05 migrates them onto
//! `RequestSpec`, this adapter lets a `ZaiClient` hand them the shared
//! `reqwest::Client` + secret + validated endpoints so they do not each
//! construct their own client/key copies.
//!
//! **P05 deletes this type** once every endpoint is migrated. It is intentionally
//! `pub(crate)` so it never appears in the public API.

use crate::client::secret::ApiSecret;
use crate::client::v2::endpoint::EndpointConfig;
use crate::client::v2::{HttpTransportConfig, ZaiClient};

/// Crate-private view of a [`ZaiClient`]'s shared interior, for use by the
/// not-yet-migrated legacy request types.
#[allow(dead_code)] // consumed by legacy request types during P02–P05; removed in P05.
pub(crate) struct LegacyRequestAdapter {
    pub(crate) secret: ApiSecret,
    pub(crate) endpoints: EndpointConfig,
    pub(crate) transport: HttpTransportConfig,
    pub(crate) reqwest: reqwest::Client,
}

impl LegacyRequestAdapter {
    #![allow(dead_code)] // see struct note.
    /// Borrow the shared interior of a [`ZaiClient`].
    pub(crate) fn from_client(client: &ZaiClient) -> LegacyRequestAdapterBorrow<'_> {
        LegacyRequestAdapterBorrow {
            secret: client.secret(),
            endpoints: client.endpoints(),
            transport: client.transport(),
            reqwest: client.reqwest(),
        }
    }

    /// Clone the interior into an owned adapter (the legacy types need owned
    /// key strings; this is the one tolerated copy site, removed in P05).
    pub(crate) fn from_client_owned(client: &ZaiClient) -> Self {
        Self {
            secret: client.secret().clone(),
            endpoints: client.endpoints().clone(),
            transport: client.transport().clone(),
            reqwest: client.reqwest().clone(),
        }
    }
}

/// Borrowed view, for zero-copy access.
#[allow(dead_code)] // consumed during P02–P05; removed in P05.
pub(crate) struct LegacyRequestAdapterBorrow<'a> {
    pub(crate) secret: &'a ApiSecret,
    pub(crate) endpoints: &'a EndpointConfig,
    pub(crate) transport: &'a HttpTransportConfig,
    pub(crate) reqwest: &'a reqwest::Client,
}
