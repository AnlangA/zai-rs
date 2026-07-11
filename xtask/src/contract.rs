//! Contract verification and manifest generation.
//!
//! This module backs the `xtask contract {verify,generate,check}` commands.
//!
//! - `verify`: re-hashes every blob listed in `spec/upstream/SOURCES.toml` and
//!   confirms its byte length and SHA-256 match the frozen values, then checks
//!   the operation/path counts against the plan's fixed values (59 operations,
//!   53 paths). This is the gate that proves the upstream snapshots have not
//!   drifted.
//! - `generate`: emits `spec/contracts/operations.json` from the frozen OpenAPI
//!   snapshot plus the fixed Rust mapping table (plan section 14).
//! - `check`: regenerates the manifest into memory and diffs it against the
//!   committed `spec/contracts/operations.json`; any drift fails.

use crate::ExitCode;
use crate::hash::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository root (parent of the xtask crate).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is always a workspace member under the repo root")
        .to_path_buf()
}

// ---------------------------------------------------------------------------
// SOURCES.toml schema (a relaxed mirror of spec/upstream/SOURCES.toml).
// Only the fields xtask needs are parsed; unknown tables are ignored.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SourcesFile {
    openapi: Vec<NamedSource>,
    #[serde(default)]
    manual: Vec<NamedSource>,
    #[serde(default)]
    coding_plan: Vec<NamedSource>,
}

#[derive(Debug, Deserialize)]
struct NamedSource {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    url: Option<String>,
    path: String,
    byte_length: u64,
    sha256: String,
    #[serde(default)]
    #[allow(dead_code)]
    path_count: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    operation_count: Option<u32>,
}

/// Entry point for `contract verify`.
pub fn verify(require_covered: bool) -> ExitCode {
    let root = repo_root();
    let sources_path = root.join("spec/upstream/SOURCES.toml");
    let sources_text = match fs::read_to_string(&sources_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", sources_path.display());
            return ExitCode::FAILURE;
        },
    };
    let sources: SourcesFile = match toml::from_str(&sources_text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot parse {}: {e}", sources_path.display());
            return ExitCode::FAILURE;
        },
    };

    let mut failures = 0u32;
    let mut checked = 0u32;
    for src in sources
        .openapi
        .iter()
        .chain(sources.manual.iter())
        .chain(sources.coding_plan.iter())
    {
        let abs = root.join(&src.path);
        let bytes = match fs::read(&abs) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("FAIL {} ({}): {e}", src.name, src.path);
                failures += 1;
                continue;
            },
        };
        let (len, hash) = (bytes.len() as u64, sha256_hex(&bytes));
        checked += 1;
        let len_ok = len == src.byte_length;
        let hash_ok = hash == src.sha256;
        if len_ok && hash_ok {
            println!("ok    {} ({} bytes)", src.name, len);
        } else {
            eprintln!(
                "FAIL  {}: expected {} bytes / {}, got {} bytes / {}",
                src.name, src.byte_length, src.sha256, len, hash
            );
            failures += 1;
        }

        // Cross-check OpenAPI path/operation counts against the plan's fixed
        // values (53 paths, 59 operations). These counts are part of the
        // contract, not metadata.
        if let Some(expected_ops) = src.operation_count {
            if let Ok(text) = fs::read_to_string(&abs) {
                if let Ok(spec) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
                        let pc = paths.len() as u32;
                        let oc = paths
                            .values()
                            .filter_map(|m| m.as_object())
                            .map(|m| {
                                m.keys()
                                    .filter(|k| {
                                        matches!(
                                            k.as_str(),
                                            "get"
                                                | "post"
                                                | "put"
                                                | "delete"
                                                | "patch"
                                                | "options"
                                                | "head"
                                        )
                                    })
                                    .count() as u32
                            })
                            .sum::<u32>();
                        if pc != src.path_count.unwrap_or(pc) || oc != expected_ops {
                            eprintln!(
                                "FAIL  {}: path/operation count mismatch (paths {pc}, ops {oc})",
                                src.name
                            );
                            failures += 1;
                        }
                    }
                }
            }
        }
    }

    println!("\nverified {checked} blobs, {failures} failure(s)");

    // If --require-covered is set, additionally confirm every operation in
    // operations.json / coverage.toml is marked covered (used by P06+).
    if require_covered {
        match coverage_all_covered(&root) {
            Ok(()) => println!("coverage: all operations covered"),
            Err(e) => {
                eprintln!("coverage: {e}");
                failures += 1;
            },
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn coverage_all_covered(root: &Path) -> Result<(), String> {
    let path = root.join("spec/contracts/coverage.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    #[derive(Deserialize)]
    struct Cov {
        #[serde(default, rename = "openapi_operation")]
        openapi: Vec<CovOp>,
        #[serde(default, rename = "coding_plan_operation")]
        coding_plan: Vec<CovOp>,
        #[serde(default, rename = "realtime_path")]
        realtime: Vec<CovOp>,
    }
    #[derive(Deserialize)]
    struct CovOp {
        operation_id: Option<String>,
        name: Option<String>,
        #[serde(default)]
        status: Option<String>,
    }
    let cov: Cov = toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let all = cov
        .openapi
        .iter()
        .chain(cov.coding_plan.iter())
        .chain(cov.realtime.iter());
    let missing: Vec<String> = all
        .filter(|o| o.status.as_deref() != Some("covered"))
        .map(|o| {
            o.operation_id
                .clone()
                .or_else(|| o.name.clone())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing coverage for: {}", missing.join(", ")))
    }
}

// ---------------------------------------------------------------------------
// operations.json generation.
// ---------------------------------------------------------------------------

/// A single serialized operation in spec/contracts/operations.json.
///
/// Fields are fixed by plan section 8 / P00.6 and section 14's mapping table.
#[derive(Debug, Clone, Serialize)]
pub struct OperationManifest {
    pub source: String,
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub api_family: String,
    pub request_content_type: String,
    pub accept: String,
    pub success_statuses: Vec<u16>,
    pub auth: String,
    pub request_schema: SchemaRef,
    pub success_schema: SchemaRef,
    pub error_schema: SchemaRef,
    pub response_mode: String,
    pub requires_done: bool,
    pub retry_safety: String,
    pub success_invariant: String,
    pub service_method: String,
    pub request_type: String,
    pub response_type: String,
    pub stream_item: Option<String>,
    pub open_map_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaRef {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
}

pub fn generate() -> ExitCode {
    let root = repo_root();
    match generate_manifest(&root) {
        Ok(ops) => {
            let out_dir = root.join("spec/contracts");
            if let Err(e) = fs::create_dir_all(&out_dir) {
                eprintln!("error: cannot create {}: {e}", out_dir.display());
                return ExitCode::FAILURE;
            }
            // Stable, sorted serialization: sort by (api_family, path, method).
            let mut sorted = ops.clone();
            sorted.sort_by(|a, b| {
                a.api_family
                    .cmp(&b.api_family)
                    .then_with(|| a.path.cmp(&b.path))
                    .then_with(|| a.method.cmp(&b.method))
            });
            let pretty = serde_json::to_string_pretty(&sorted).unwrap();
            let out_path = out_dir.join("operations.json");
            if let Err(e) = fs::write(&out_path, pretty + "\n") {
                eprintln!("error: cannot write {}: {e}", out_path.display());
                return ExitCode::FAILURE;
            }
            println!("wrote {} ({} operations)", out_path.display(), sorted.len());
            ExitCode::SUCCESS
        },
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        },
    }
}

pub fn check() -> ExitCode {
    let root = repo_root();
    let generated = match generate_manifest(&root) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        },
    };
    let mut sorted = generated.clone();
    sorted.sort_by(|a, b| {
        a.api_family
            .cmp(&b.api_family)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.method.cmp(&b.method))
    });
    let regenerated = serde_json::to_string_pretty(&sorted).unwrap() + "\n";

    let out_path = root.join("spec/contracts/operations.json");
    let committed = match fs::read_to_string(&out_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", out_path.display());
            return ExitCode::FAILURE;
        },
    };
    if committed == regenerated {
        println!("operations.json up to date ({} operations)", sorted.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("operations.json is stale; run `cargo run -p xtask -- contract generate`");
        ExitCode::FAILURE
    }
}

/// Build the full operation manifest from the frozen OpenAPI snapshot plus the
/// fixed Rust mapping table (plan section 14). The mapping table is the single
/// source of truth for operation_id / service_method / request_type /
/// response_type; OpenAPI supplies method/path/content-type/status/schema refs.
fn generate_manifest(root: &Path) -> Result<Vec<OperationManifest>, String> {
    let openapi_path = root.join("spec/upstream/openapi-2026-07-11.json");
    let text = fs::read_to_string(&openapi_path)
        .map_err(|e| format!("read {}: {e}", openapi_path.display()))?;
    let spec: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse openapi: {e}"))?;
    let paths = spec
        .get("paths")
        .and_then(|p| p.as_object())
        .ok_or_else(|| "openapi has no paths object".to_string())?;

    // Index: "METHOD path" -> raw operation object.
    let mut by_key: BTreeMap<String, &serde_json::Value> = BTreeMap::new();
    for (path, item) in paths {
        let Some(methods) = item.as_object() else {
            continue;
        };
        for (method, op) in methods {
            if !matches!(
                method.as_str(),
                "get" | "post" | "put" | "delete" | "patch" | "options" | "head"
            ) {
                continue;
            }
            by_key.insert(format!("{} {}", method.to_uppercase(), path), op);
        }
    }

    let table = mapping::fixed_table();
    let mut out = Vec::with_capacity(table.len());
    for entry in &table {
        let key = format!("{} {}", entry.method, entry.path);
        let op = by_key.get(&key).ok_or_else(|| {
            format!("mapping table references {key} which is absent from frozen OpenAPI")
        })?;

        let request_content_type = extract_request_content_type(op);
        let success_statuses = extract_success_statuses(op);
        let request_schema = extract_request_schema(op);
        let success_schema = extract_success_schema(op);

        out.push(OperationManifest {
            source: "openapi-2026-07-11".to_string(),
            operation_id: entry.operation_id.to_string(),
            method: entry.method.to_string(),
            path: entry.path.to_string(),
            api_family: entry.api_family.to_string(),
            request_content_type,
            accept: entry.accept.to_string(),
            success_statuses,
            auth: "bearer".to_string(),
            request_schema,
            success_schema,
            error_schema: SchemaRef {
                kind: "ref".to_string(),
                name: "ApiErrorEnvelope".to_string(),
            },
            response_mode: entry.response_mode.to_string(),
            requires_done: entry.requires_done,
            retry_safety: entry.retry_safety.to_string(),
            success_invariant: entry.success_invariant.to_string(),
            service_method: entry.service_method.to_string(),
            request_type: entry.request_type.to_string(),
            response_type: entry.response_type.to_string(),
            stream_item: entry.stream_item.map(|s| s.to_string()),
            open_map_fields: entry
                .open_map_fields
                .iter()
                .map(|s| s.to_string())
                .collect(),
        });
    }

    // Guard: the manifest must enumerate exactly the 59 operations present in
    // the frozen snapshot — no more, no less.
    let op_count: u32 = paths
        .values()
        .filter_map(|m| m.as_object())
        .map(|m| {
            m.keys()
                .filter(|k| {
                    matches!(
                        k.as_str(),
                        "get" | "post" | "put" | "delete" | "patch" | "options" | "head"
                    )
                })
                .count() as u32
        })
        .sum();
    if out.len() as u32 != op_count {
        return Err(format!(
            "manifest has {} entries but OpenAPI has {op_count} operations",
            out.len()
        ));
    }

    Ok(out)
}

fn extract_request_content_type(op: &serde_json::Value) -> String {
    op.get("requestBody")
        .and_then(|rb| rb.get("content"))
        .and_then(|c| c.as_object())
        .and_then(|o| {
            o.keys().next().map(|k| {
                if k.starts_with("multipart/") {
                    "multipart/form-data".to_string()
                } else {
                    k.clone()
                }
            })
        })
        .unwrap_or_else(|| "application/json".to_string())
}

fn extract_success_statuses(op: &serde_json::Value) -> Vec<u16> {
    let mut statuses = Vec::new();
    if let Some(responses) = op.get("responses").and_then(|r| r.as_object()) {
        for code in responses.keys() {
            if let Ok(n) = code.parse::<u16>() {
                if (200..300).contains(&n) {
                    statuses.push(n);
                }
            }
        }
    }
    statuses.sort();
    if statuses.is_empty() {
        vec![200]
    } else {
        statuses
    }
}

fn extract_request_schema(op: &serde_json::Value) -> SchemaRef {
    let rb = op
        .get("requestBody")
        .and_then(|rb| rb.get("content"))
        .and_then(|c| c.get("application/json"))
        .or_else(|| {
            op.get("requestBody")
                .and_then(|rb| rb.get("content"))
                .and_then(|c| c.get("multipart/form-data"))
        });
    schema_ref_of(rb.and_then(|x| x.get("schema")))
}

fn extract_success_schema(op: &serde_json::Value) -> SchemaRef {
    let success = op
        .get("responses")
        .and_then(|r| {
            r.as_object()
                .and_then(|o| o.keys().find(|k| k.starts_with('2')).and_then(|k| o.get(k)))
        })
        .and_then(|s| s.get("content"))
        .and_then(|c| c.get("application/json"))
        .or_else(|| {
            // Some binary responses only advertise */* or octet-stream.
            op.get("responses")
                .and_then(|r| r.as_object())
                .and_then(|o| o.keys().find(|k| k.starts_with('2')).and_then(|k| o.get(k)))
                .and_then(|s| s.get("content"))
                .and_then(|c| c.as_object())
                .and_then(|o| o.values().next())
        });
    schema_ref_of(success.and_then(|x| x.get("schema")))
}

fn schema_ref_of(schema: Option<&serde_json::Value>) -> SchemaRef {
    if let Some(s) = schema {
        // oneOf with a single $ref → use that ref's name.
        if let Some(oneof) = s.get("oneOf").and_then(|o| o.as_array()) {
            if let Some(first) = oneof
                .first()
                .and_then(|v| v.get("$ref").and_then(|r| r.as_str()))
            {
                return SchemaRef {
                    kind: "ref".to_string(),
                    name: ref_name(first).to_string(),
                };
            }
        }
        if let Some(rf) = s.get("$ref").and_then(|r| r.as_str()) {
            return SchemaRef {
                kind: "ref".to_string(),
                name: ref_name(rf).to_string(),
            };
        }
        if let Some(t) = s.get("type").and_then(|t| t.as_str()) {
            return SchemaRef {
                kind: t.to_string(),
                name: t.to_string(),
            };
        }
    }
    SchemaRef {
        kind: "none".to_string(),
        name: "none".to_string(),
    }
}

fn ref_name(rf: &str) -> &str {
    rf.rsplit('/').next().unwrap_or(rf)
}

/// The fixed Rust mapping table (plan section 14). This is the authoritative
/// mapping from (method, path) → operation_id / service method / request and
/// response types. It is hand-encoded from the plan and must not be derived
/// from a live web page.
mod mapping {
    pub struct Entry {
        pub method: &'static str,
        pub path: &'static str,
        pub operation_id: &'static str,
        pub api_family: &'static str,
        pub accept: &'static str,
        pub response_mode: &'static str,
        pub requires_done: bool,
        pub retry_safety: &'static str,
        pub success_invariant: &'static str,
        pub service_method: &'static str,
        pub request_type: &'static str,
        pub response_type: &'static str,
        pub stream_item: Option<&'static str>,
        pub open_map_fields: &'static [&'static str],
    }

    pub const fn idem() -> &'static str {
        "Idempotent"
    }
    pub const fn nonidem() -> &'static str {
        "NonIdempotent"
    }

    pub fn fixed_table() -> Vec<Entry> {
        // Invariant text shared across typed responses.
        let inv = "all OpenAPI-required fields present and at least one documented response field non-null";
        vec![
            Entry {
                method: "GET",
                path: "/llm-application/open/document",
                operation_id: "knowledge.list_documents",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.knowledge().list_documents",
                request_type: "DocumentListRequest",
                response_type: "DocumentListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/document/embedding/{id}",
                operation_id: "knowledge.reembed_document",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.knowledge().reembed_document",
                request_type: "DocumentReembedRequest",
                response_type: "DocumentReembedResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/document/slice/image_list/{id}",
                operation_id: "knowledge.list_document_images",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.knowledge().list_document_images",
                request_type: "DocumentImageListRequest",
                response_type: "DocumentImageListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/document/upload_document/{id}",
                operation_id: "knowledge.upload_document",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.knowledge().upload_document",
                request_type: "DocumentUploadRequest",
                response_type: "DocumentUploadResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/document/upload_url",
                operation_id: "knowledge.upload_document_url",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.knowledge().upload_document_url",
                request_type: "DocumentUrlUploadRequest",
                response_type: "DocumentUrlUploadResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "DELETE",
                path: "/llm-application/open/document/{id}",
                operation_id: "knowledge.delete_document",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.knowledge().delete_document",
                request_type: "DocumentDeleteRequest",
                response_type: "DocumentDeleteResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/llm-application/open/document/{id}",
                operation_id: "knowledge.get_document",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.knowledge().get_document",
                request_type: "DocumentGetRequest",
                response_type: "DocumentGetResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/llm-application/open/history_session_record/{app_id}/{conversation_id}",
                operation_id: "applications.history",
                api_family: "ApplicationV2",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.applications().history",
                request_type: "ApplicationHistoryRequest",
                response_type: "ApplicationHistoryResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/llm-application/open/knowledge",
                operation_id: "knowledge.list",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().list",
                request_type: "KnowledgeListRequest",
                response_type: "KnowledgeListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/knowledge",
                operation_id: "knowledge.create",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().create",
                request_type: "KnowledgeCreateRequest",
                response_type: "KnowledgeCreateResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/llm-application/open/knowledge/capacity",
                operation_id: "knowledge.capacity",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().capacity",
                request_type: "KnowledgeCapacityRequest",
                response_type: "KnowledgeCapacityResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/knowledge/retrieve",
                operation_id: "knowledge.retrieve",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().retrieve",
                request_type: "KnowledgeSearchRequest",
                response_type: "KnowledgeSearchResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "DELETE",
                path: "/llm-application/open/knowledge/{id}",
                operation_id: "knowledge.delete",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().delete",
                request_type: "KnowledgeDeleteRequest",
                response_type: "KnowledgeDeleteResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/llm-application/open/knowledge/{id}",
                operation_id: "knowledge.get",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().get",
                request_type: "KnowledgeGetRequest",
                response_type: "KnowledgeGetResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "PUT",
                path: "/llm-application/open/knowledge/{id}",
                operation_id: "knowledge.update",
                api_family: "LlmApplication",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: "envelope code == 200 and data present",
                service_method: "client.knowledge().update",
                request_type: "KnowledgeUpdateRequest",
                response_type: "KnowledgeUpdateResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/v2/application/file_stat",
                operation_id: "applications.file_stats",
                api_family: "ApplicationV2",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.applications().file_stats",
                request_type: "ApplicationFileStatsRequest",
                response_type: "ApplicationFileStatsResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/v2/application/file_upload",
                operation_id: "applications.upload_file",
                api_family: "ApplicationV2",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.applications().upload_file",
                request_type: "ApplicationFileUploadRequest",
                response_type: "ApplicationFileUploadResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/v2/application/slice_info",
                operation_id: "applications.slice_info",
                api_family: "ApplicationV2",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.applications().slice_info",
                request_type: "ApplicationSliceInfoRequest",
                response_type: "ApplicationSliceInfoResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/v2/application/{app_id}/conversation",
                operation_id: "applications.create_conversation",
                api_family: "ApplicationV2",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.applications().create_conversation",
                request_type: "ApplicationConversationCreateRequest",
                response_type: "ApplicationConversationCreateResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/llm-application/open/v2/application/{app_id}/variables",
                operation_id: "applications.variables",
                api_family: "ApplicationV2",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.applications().variables",
                request_type: "ApplicationVariablesRequest",
                response_type: "ApplicationVariablesResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/llm-application/open/v3/application/invoke",
                operation_id: "applications.invoke",
                api_family: "ApplicationV3",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.applications().invoke",
                request_type: "ApplicationInvokeRequest",
                response_type: "ApplicationInvokeResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/assistant",
                operation_id: "assistants.invoke",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.assistants().invoke",
                request_type: "AssistantInvokeRequest",
                response_type: "AssistantInvokeResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/assistant/conversation/list",
                operation_id: "assistants.conversations",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.assistants().conversations",
                request_type: "AssistantConversationListRequest",
                response_type: "AssistantConversationListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/assistant/list",
                operation_id: "assistants.list",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.assistants().list",
                request_type: "AssistantListRequest",
                response_type: "AssistantListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/async-result/{id}",
                operation_id: "tasks.get",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.tasks().get",
                request_type: "AsyncTaskGetRequest",
                response_type: "AsyncTaskResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/async/chat/completions",
                operation_id: "chat.complete_async",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.chat().complete_async",
                request_type: "AsyncChatRequest",
                response_type: "AsyncTaskResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/async/images/generations",
                operation_id: "images.generate_async",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.images().generate_async",
                request_type: "AsyncImageGenerationRequest",
                response_type: "AsyncTaskResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/audio/speech",
                operation_id: "audio.synthesize",
                api_family: "PaasV4",
                accept: "application/octet-stream",
                response_mode: "binary_or_stream",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.audio().synthesize / synthesize_stream",
                request_type: "TextToSpeechRequest",
                response_type: "Bytes / AudioByteStream",
                stream_item: Some("AudioByteStream"),
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/audio/transcriptions",
                operation_id: "audio.transcribe",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json_or_stream",
                requires_done: true,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.audio().transcribe / transcribe_stream",
                request_type: "SpeechToTextRequest",
                response_type: "SpeechToTextResponse / SpeechToTextStream",
                stream_item: Some("SpeechToTextEvent"),
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/batches",
                operation_id: "batches.list",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.batches().list",
                request_type: "BatchListRequest",
                response_type: "BatchListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/batches",
                operation_id: "batches.create",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.batches().create",
                request_type: "BatchCreateRequest",
                response_type: "BatchCreateResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/batches/{batch_id}",
                operation_id: "batches.get",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.batches().get",
                request_type: "BatchGetRequest",
                response_type: "BatchGetResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/batches/{batch_id}/cancel",
                operation_id: "batches.cancel",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.batches().cancel",
                request_type: "BatchCancelRequest",
                response_type: "BatchCancelResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/chat/completions",
                operation_id: "chat.complete",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json_or_stream",
                requires_done: true,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.chat().complete / stream",
                request_type: "ChatRequest",
                response_type: "ChatResponse / ChatEventStream",
                stream_item: Some("ChatEvent"),
                open_map_fields: &["tools", "response_format"],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/embeddings",
                operation_id: "embeddings.create",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.embeddings().create",
                request_type: "EmbeddingRequest",
                response_type: "EmbeddingResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/files",
                operation_id: "files.list",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.files().list",
                request_type: "FileListRequest",
                response_type: "FileListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/files",
                operation_id: "files.upload",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.files().upload",
                request_type: "FileUploadRequest",
                response_type: "FileUploadResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/files/ocr",
                operation_id: "files.ocr",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.files().ocr",
                request_type: "OcrRequest",
                response_type: "OcrResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/files/parser/create",
                operation_id: "files.parse",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.files().parse",
                request_type: "FileParseRequest",
                response_type: "AsyncTaskResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/files/parser/result/{taskId}/{format_type}",
                operation_id: "files.parse_result",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.files().parse_result",
                request_type: "FileParseResultRequest",
                response_type: "FileParseResultResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/files/parser/sync",
                operation_id: "files.parse_sync",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.files().parse_sync",
                request_type: "FileParseSyncRequest",
                response_type: "FileResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "DELETE",
                path: "/paas/v4/files/{file_id}",
                operation_id: "files.delete",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.files().delete",
                request_type: "FileDeleteRequest",
                response_type: "FileDeleteResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/files/{file_id}/content",
                operation_id: "files.content",
                api_family: "PaasV4",
                accept: "application/octet-stream",
                response_mode: "binary_stream",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.files().content",
                request_type: "FileContentRequest",
                response_type: "ByteStream",
                stream_item: Some("Bytes"),
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/images/generations",
                operation_id: "images.generate",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.images().generate",
                request_type: "ImageGenerationRequest",
                response_type: "ImageGenerationResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/layout_parsing",
                operation_id: "tools.parse_layout",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.tools().parse_layout",
                request_type: "LayoutParsingRequest",
                response_type: "LayoutParsingResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/moderations",
                operation_id: "moderation.check",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.moderation().check",
                request_type: "ModerationRequest",
                response_type: "ModerationResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/reader",
                operation_id: "tools.read_document",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.tools().read_document",
                request_type: "ReaderRequest",
                response_type: "ReaderResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/rerank",
                operation_id: "rerank.create",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.rerank().create",
                request_type: "RerankRequest",
                response_type: "RerankResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/tokenizer",
                operation_id: "tokenizer.count",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.tokenizer().count",
                request_type: "TokenizerRequest",
                response_type: "TokenizerResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/videos/generations",
                operation_id: "videos.generate",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.videos().generate",
                request_type: "VideoGenerationRequest",
                response_type: "AsyncTaskResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/voice/clone",
                operation_id: "audio.clone_voice",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.audio().clone_voice",
                request_type: "VoiceCloneRequest",
                response_type: "VoiceCloneResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/voice/delete",
                operation_id: "audio.delete_voice",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.audio().delete_voice",
                request_type: "VoiceDeleteRequest",
                response_type: "VoiceDeleteResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "GET",
                path: "/paas/v4/voice/list",
                operation_id: "audio.list_voices",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: idem(),
                success_invariant: inv,
                service_method: "client.audio().list_voices",
                request_type: "VoiceListRequest",
                response_type: "VoiceListResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/paas/v4/web_search",
                operation_id: "tools.web_search",
                api_family: "PaasV4",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.tools().web_search",
                request_type: "WebSearchRequest",
                response_type: "WebSearchResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/v1/agents",
                operation_id: "agents.invoke",
                api_family: "AgentV1",
                accept: "application/json",
                response_mode: "json_or_stream",
                requires_done: true,
                retry_safety: nonidem(),
                success_invariant: "Completed: id+agent_id+non-empty choices; Pending: agent_id+async_id",
                service_method: "client.agents().invoke / stream",
                request_type: "AgentInvokeRequest",
                response_type: "AgentInvokeResponse / AgentEventStream",
                stream_item: Some("AgentEvent"),
                open_map_fields: &["custom_variables"],
            },
            Entry {
                method: "POST",
                path: "/v1/agents/async-result",
                operation_id: "agents.async_result",
                api_family: "AgentV1",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: "Pending: agent_id+async_id; Succeeded: +non-empty choices",
                service_method: "client.agents().async_result",
                request_type: "AgentAsyncResultRequest",
                response_type: "AgentAsyncResult",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/v1/agents/conversation",
                operation_id: "agents.conversation",
                api_family: "AgentV1",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: "conversation_id+agent_id+non-empty choices",
                service_method: "client.agents().conversation",
                request_type: "AgentConversationRequest",
                response_type: "AgentConversationResponse",
                stream_item: None,
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/zrag/agent/chat",
                operation_id: "zrag.chat",
                api_family: "Zrag",
                accept: "text/event-stream",
                response_mode: "stream",
                requires_done: true,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.zrag().chat",
                request_type: "ZragChatRequest",
                response_type: "ZragEventStream",
                stream_item: Some("AgentStreamEvent"),
                open_map_fields: &[],
            },
            Entry {
                method: "POST",
                path: "/zrag/retrieval/retrieve",
                operation_id: "zrag.retrieve",
                api_family: "Zrag",
                accept: "application/json",
                response_mode: "json",
                requires_done: false,
                retry_safety: nonidem(),
                success_invariant: inv,
                service_method: "client.zrag().retrieve",
                request_type: "ZragRetrieveRequest",
                response_type: "ZragRetrieveResponse",
                stream_item: None,
                open_map_fields: &[],
            },
        ]
    }
}
