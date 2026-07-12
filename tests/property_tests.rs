//! Property-based tests for endpoint encoding and SSE parsing.
//!
//! Uses `proptest` to cover:
//! - dynamic path/query percent-encoding
//! - SSE chunk-split resilience (arbitrary boundaries)

use proptest::prelude::*;

use zai_rs::model::sse_parser::SseEventParser;

// ---------------------------------------------------------------------------
// Path-segment encoding.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn endpoint_resolve_percent_encodes_special_chars(s in "[a-zA-Z0-9_.%/?#]{1,20}") {
        let ec = zai_rs::client::endpoint::EndpointConfig::defaults().unwrap();
        prop_assume!(s != "." && s != "..");
        let resolved = ec.resolve(zai_rs::client::ApiFamily::PaasV4, &[&s]).unwrap();
        prop_assert!(url::Url::parse(&resolved).is_ok());
        prop_assert!(resolved.starts_with(ec.base(zai_rs::client::ApiFamily::PaasV4).as_str()));
        if s.contains('/') {
            prop_assert!(resolved.contains("%2F"));
        }
    }
}

// ---------------------------------------------------------------------------
// SSE chunk-split resilience.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn sse_parse_recovers_from_arbitrary_split(
        payload in "[^\r\n]{1,50}",
        split_seed in any::<usize>(),
    ) {
        let data = format!("data: {payload}\n\n");
        let mut p = SseEventParser::new();
        let split = split_seed % (data.len() + 1);
        let mut events = p.push(&data.as_bytes()[..split]);
        events.extend(p.push(&data.as_bytes()[split..]));
        prop_assert_eq!(events, vec![payload.into_bytes()]);
    }
}
