//! Tracing and log-redaction tests.
//!
//! The SDK instruments every HTTP request/response with `tracing::trace!`
//! lines. The request body and response body are passed through
//! [`mask_sensitive_info`] before being recorded, and the `Authorization`
//! header value is *never* a tracing field (only reqwest's `bearer_auth()`
//! attaches it to the outgoing request). These tests pin that invariant: an
//! API key, `Authorization`, and `Bearer` must not appear in the masked output
//! that tracing records, even when the body legitimately contains the key.

use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use zai_rs::client::error::mask_sensitive_info;

/// A realistic Zhipu key has the shape `<id>.<secret>` where the secret is
/// base64-ish. The masking regex targets this shape.
const TEST_KEY: &str = "1234567890.abcdefghijklmnop";

#[test]
fn mask_redacts_api_key_in_body() {
    // A request body that echoes the key.
    let body = format!(r#"{{"api_key":"{TEST_KEY}","prompt":"hello"}}"#);
    let masked = mask_sensitive_info(&body);
    assert!(
        !masked.contains(TEST_KEY),
        "API key leaked through mask: {masked}"
    );
    assert!(
        masked.contains("[FILTERED]"),
        "mask did not substitute a filter marker: {masked}"
    );
}

#[test]
fn mask_redacts_authorization_header_line() {
    let line = format!("Authorization: Bearer {TEST_KEY}");
    let masked = mask_sensitive_info(&line);
    assert!(
        !masked.contains(TEST_KEY),
        "key leaked from Authorization line: {masked}"
    );
    assert!(
        !masked.contains("Bearer "),
        "Bearer token marker leaked: {masked}"
    );
}

/// Capture every tracing event emitted while `f` runs into the returned
/// `String`. This lets us assert on the *actual* text the SDK would log.
fn capture_tracing<F: FnOnce()>(f: F) -> String {
    use std::sync::{Arc, Mutex};
    let buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let buf2 = buf.clone();

    struct CaptureLayer {
        buf: Arc<Mutex<String>>,
    }
    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = RecVisitor {
                rendered: String::new(),
            };
            event.record(&mut visitor);
            let mut b = self.buf.lock().unwrap();
            b.push_str(&visitor.rendered);
            b.push('\n');
        }
    }
    struct RecVisitor {
        rendered: String,
    }
    impl tracing::field::Visit for RecVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.rendered
                .push_str(&format!("{}={:?} ", field.name(), value));
        }
    }

    let layer = CaptureLayer { buf: buf2 };
    let _guard = tracing_subscriber::registry().with(layer).set_default();
    f();
    // Drop the dispatcher guard first so no further writes race, then clone the
    // buffer out (it is still behind an Arc the layer captured).
    drop(_guard);
    buf.lock().unwrap().clone()
}

#[test]
fn sdk_trace_events_never_contain_key_or_authorization() {
    // Emit trace events exactly as the SDK does: masked request_body /
    // response_body, method+url metadata. The SDK's chat request body never
    // carries an `Authorization` field (that's an HTTP header attached by
    // `bearer_auth`, not a tracing field), but a malicious/proxy response body
    // could echo one — so the masked payload must still strip both the key and
    // any Authorization: Bearer header line.
    let body = format!(
        r#"{{"model":"glm-5.2","messages":[{{"role":"user","content":"hi"}}],"key":"{TEST_KEY}"}}
Authorization: Bearer {TEST_KEY}"#
    );
    let masked_body = mask_sensitive_info(&body);

    let captured = capture_tracing(|| {
        tracing::trace!(
            method = "POST",
            url = "https://open.bigmodel.cn/api/paas/v4/chat/completions",
            request_body = %masked_body,
            "Sending HTTP request body"
        );
    });

    assert!(
        !captured.contains(TEST_KEY),
        "API key leaked into tracing output: {captured}"
    );
    assert!(
        !captured.contains("Bearer "),
        "Bearer token leaked into tracing output: {captured}"
    );
    assert!(
        !captured.contains("Authorization:"),
        "Authorization header leaked into tracing output: {captured}"
    );
}
