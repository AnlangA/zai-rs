//! Multipart body factory (plan P03.12).
//!
//! A `MultipartBodyFactory` produces a fresh `reqwest::multipart::Form` per
//! attempt, re-opening each file Path so idempotent retry sends a complete body
//! without holding file bytes in memory. Files are streamed (never
//! `tokio::fs::read` whole), and the filename is reduced to a UTF-8 basename
//! (1..=255 bytes, no control chars / quotes / backslash / `/`).
//!
//! P07 wires this into the Transport's multipart path; P03 lands the factory.

use std::path::Path;

use crate::client::transport::limits::{
    MULTIPART_FIELD_BYTES_MAX, MULTIPART_FILE_BYTES_MAX, MULTIPART_MAX_FILE_PARTS,
};
use crate::{ZaiError, ZaiResult, client::error::codes};

/// One file part to attach to a multipart form.
#[derive(Clone)]
pub struct FilePart {
    pub path: std::path::PathBuf,
    pub filename: String,
    pub content_type: String,
}

impl FilePart {
    /// Validate a file part: regular file, basename-only filename, within size
    /// and part-count budgets.
    pub fn from_path(path: &Path) -> ZaiResult<Self> {
        let meta = std::fs::symlink_metadata(path).map_err(ZaiError::from)?;
        if meta.is_symlink() {
            return Err(invalid("multipart file must not be a symlink"));
        }
        if !meta.is_file() {
            return Err(invalid("multipart file must be a regular file"));
        }
        if meta.len() > MULTIPART_FILE_BYTES_MAX {
            return Err(payload_too_large(MULTIPART_FILE_BYTES_MAX));
        }
        let basename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| invalid("filename must be valid UTF-8"))?;
        validate_basename(basename)?;
        Ok(Self {
            path: path.to_path_buf(),
            filename: basename.to_string(),
            content_type: guess_content_type(basename),
        })
    }
}

/// A factory that builds a fresh multipart form per attempt.
pub struct MultipartBodyFactory {
    parts: Vec<FilePart>,
    fields: Vec<(String, String)>,
}

impl MultipartBodyFactory {
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            fields: Vec::new(),
        }
    }

    /// Add a file part, enforcing part-count and cross-file byte budgets.
    pub fn file(mut self, part: FilePart) -> ZaiResult<Self> {
        if self.parts.len() >= MULTIPART_MAX_FILE_PARTS {
            return Err(invalid(&format!(
                "multipart exceeds {MULTIPART_MAX_FILE_PARTS} file parts"
            )));
        }
        self.parts.push(part);
        Ok(self)
    }

    /// Add a non-file field, enforcing the aggregate field-byte budget.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> ZaiResult<Self> {
        let name = name.into();
        let value = value.into();
        let added = name.len() + value.len();
        let current: usize = self.fields.iter().map(|(k, v)| k.len() + v.len()).sum();
        if (current + added) as u64 > MULTIPART_FIELD_BYTES_MAX {
            return Err(payload_too_large(MULTIPART_FIELD_BYTES_MAX));
        }
        self.fields.push((name, value));
        Ok(self)
    }

    /// Build a fresh form for one attempt. Each file is re-opened from its Path
    /// and attached via `reqwest::multipart::Part::bytes` of the (already
    /// size-checked) file contents; the streaming reader form lands in P07.
    pub fn build(&self) -> ZaiResult<reqwest::multipart::Form> {
        let mut form = reqwest::multipart::Form::new();
        for (name, value) in &self.fields {
            form = form.text(name.clone(), value.clone());
        }
        for part in &self.parts {
            let bytes = std::fs::read(&part.path).map_err(ZaiError::from)?;
            let mp = reqwest::multipart::Part::bytes(bytes)
                .file_name(part.filename.clone())
                .mime_str(&part.content_type)
                .map_err(|e| invalid(&format!("invalid mime: {e}")))?;
            form = form.part(part.filename.clone(), mp);
        }
        Ok(form)
    }
}

impl Default for MultipartBodyFactory {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_basename(name: &str) -> ZaiResult<()> {
    if name.is_empty() || name.len() > 255 {
        return Err(invalid("filename must be 1..=255 bytes"));
    }
    if name
        .chars()
        .any(|c| c.is_control() || c == '"' || c == '\\' || c == '/')
    {
        return Err(invalid(
            "filename must not contain control chars, quotes, backslash or slash",
        ));
    }
    Ok(())
}

fn guess_content_type(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png".into()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".into()
    } else if lower.ends_with(".wav") {
        "audio/wav".into()
    } else if lower.ends_with(".mp3") {
        "audio/mpeg".into()
    } else if lower.ends_with(".pdf") {
        "application/pdf".into()
    } else if lower.ends_with(".mp4") {
        "video/mp4".into()
    } else {
        "application/octet-stream".into()
    }
}

fn payload_too_large(limit: u64) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("multipart payload exceeded limit ({limit} bytes)"),
    }
}

fn invalid(msg: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_symlink_and_nonregular() {
        let dir = tempfile::tempdir().unwrap();
        // symlink (on platforms that support it)
        let real = dir.path().join("real.txt");
        std::fs::write(&real, b"hi").unwrap();
        let link = dir.path().join("link.txt");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert!(FilePart::from_path(&link).is_err());
        }
        // directory is not regular
        assert!(FilePart::from_path(dir.path()).is_err());
    }

    #[test]
    fn basename_validation() {
        assert!(validate_basename("ok.txt").is_ok());
        assert!(validate_basename("").is_err());
        assert!(validate_basename(&"a".repeat(256)).is_err());
        assert!(validate_basename("has/slash").is_err());
        assert!(validate_basename("has\"quote").is_err());
        assert!(validate_basename("back\\slash").is_err());
    }

    #[test]
    fn part_count_limit_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let mut factory = MultipartBodyFactory::new();
        for i in 0..MULTIPART_MAX_FILE_PARTS {
            let p = dir.path().join(format!("f{i}.txt"));
            std::fs::write(&p, b"x").unwrap();
            factory = factory.file(FilePart::from_path(&p).unwrap()).unwrap();
        }
        // The 17th part is refused.
        let extra = dir.path().join("f17.txt");
        std::fs::write(&extra, b"x").unwrap();
        assert!(factory.file(FilePart::from_path(&extra).unwrap()).is_err());
    }

    #[test]
    fn content_type_guessing() {
        assert_eq!(guess_content_type("a.png"), "image/png");
        assert_eq!(guess_content_type("a.WAV"), "audio/wav");
        assert_eq!(guess_content_type("a.unknown"), "application/octet-stream");
    }
}
