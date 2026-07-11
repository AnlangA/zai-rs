//! P03 acceptance: redirect policy (plan P03 验证 — redirect_policy).

use url::Url;
use zai_rs::client::v2::transport::redirect::{MAX_REDIRECTS, follow};
use zai_rs::client::v2::transport::retry::RetrySafety;

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

#[test]
fn get_follows_301_302_303_307_308_same_origin() {
    let cur = url("https://open.bigmodel.cn/a");
    for status in [301, 302, 303, 307, 308] {
        let r = follow(&cur, status, "/b", RetrySafety::Idempotent, "GET", 0).unwrap();
        assert_eq!(
            r.unwrap().as_str(),
            "https://open.bigmodel.cn/b",
            "GET {status}"
        );
    }
}

#[test]
fn put_delete_options_follow_only_307_308() {
    let cur = url("https://open.bigmodel.cn/a");
    for method in ["PUT", "DELETE", "OPTIONS"] {
        // 301/302/303 not followed.
        for status in [301, 302, 303] {
            assert!(
                follow(&cur, status, "/b", RetrySafety::Idempotent, method, 0)
                    .unwrap()
                    .is_none(),
                "{method} must not follow {status}"
            );
        }
        // 307/308 followed.
        for status in [307, 308] {
            assert!(
                follow(&cur, status, "/b", RetrySafety::Idempotent, method, 0)
                    .unwrap()
                    .is_some(),
                "{method} should follow {status}"
            );
        }
    }
}

#[test]
fn nonidempotent_never_follows_any_3xx() {
    let cur = url("https://open.bigmodel.cn/a");
    for status in [301, 302, 303, 307, 308] {
        assert!(
            follow(&cur, status, "/b", RetrySafety::NonIdempotent, "POST", 0)
                .unwrap()
                .is_none(),
            "POST must not follow {status}"
        );
    }
}

#[test]
fn cross_origin_refused() {
    let cur = url("https://open.bigmodel.cn/a");
    let err = follow(
        &cur,
        302,
        "https://evil.example.com/b",
        RetrySafety::Idempotent,
        "GET",
        0,
    )
    .unwrap_err();
    assert!(format!("{err}").contains("cross-origin"));
}

#[test]
fn tls_downgrade_refused() {
    let cur = url("https://open.bigmodel.cn/a");
    let err = follow(
        &cur,
        302,
        "http://open.bigmodel.cn/b",
        RetrySafety::Idempotent,
        "GET",
        0,
    )
    .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("TLS downgrade") || msg.contains("cross-origin"),
        "got: {msg}"
    );
}

#[test]
fn redirect_target_must_not_carry_userinfo_or_fragment() {
    let cur = url("https://open.bigmodel.cn/a");
    // userinfo
    let err = follow(
        &cur,
        302,
        "https://u:p@open.bigmodel.cn/b",
        RetrySafety::Idempotent,
        "GET",
        0,
    )
    .unwrap_err();
    assert!(format!("{err}").contains("userinfo"));
    // fragment
    let err = follow(
        &cur,
        302,
        "https://open.bigmodel.cn/b#frag",
        RetrySafety::Idempotent,
        "GET",
        0,
    )
    .unwrap_err();
    assert!(format!("{err}").contains("fragment"));
}

#[test]
fn max_three_hops() {
    let cur = url("https://open.bigmodel.cn/a");
    assert!(
        follow(
            &cur,
            302,
            "/b",
            RetrySafety::Idempotent,
            "GET",
            MAX_REDIRECTS
        )
        .is_err()
    );
    assert!(
        follow(
            &cur,
            302,
            "/b",
            RetrySafety::Idempotent,
            "GET",
            MAX_REDIRECTS - 1
        )
        .is_ok()
    );
}

#[test]
fn non_redirect_status_returns_none() {
    let cur = url("https://open.bigmodel.cn/a");
    assert!(
        follow(&cur, 200, "/b", RetrySafety::Idempotent, "GET", 0)
            .unwrap()
            .is_none()
    );
    assert!(
        follow(&cur, 404, "/b", RetrySafety::Idempotent, "GET", 0)
            .unwrap()
            .is_none()
    );
}
