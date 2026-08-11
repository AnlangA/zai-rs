//! Multipart body factory for retryable request construction.
//!
//! A `MultipartBodyFactory` produces a fresh `reqwest::multipart::Form` per
//! attempt and asynchronously reopens every path-backed part. Files are streamed
//! from bounded handles instead of being buffered in memory. Filenames must be
//! UTF-8 basenames of 1..=255 bytes with no control characters, quotes,
//! backslashes, or slashes.

use std::path::Path;

use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;

use crate::client::transport::limits::{
    MULTIPART_FIELD_BYTES_MAX, MULTIPART_FILE_BYTES_MAX, MULTIPART_MAX_FILE_PARTS,
};
use crate::{ZaiError, ZaiResult, client::error::codes};

/// One file part to attach to a multipart form.
#[derive(Clone)]
pub struct FilePart {
    /// Local file path reopened for each form build.
    path: std::path::PathBuf,
    /// Validated basename sent in the multipart metadata.
    filename: String,
    /// MIME type attached to the part.
    content_type: String,
    identity: FileIdentity,
}

#[derive(Clone)]
struct FileIdentity {
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    change_seconds: i64,
    #[cfg(unix)]
    change_nanoseconds: i64,
}

impl FileIdentity {
    fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(unix)]
            change_seconds: metadata.ctime(),
            #[cfg(unix)]
            change_nanoseconds: metadata.ctime_nsec(),
        }
    }

    fn matches(&self, metadata: &std::fs::Metadata) -> bool {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        self.len == metadata.len() && self.modified == metadata.modified().ok() && {
            #[cfg(unix)]
            {
                self.device == metadata.dev()
                    && self.inode == metadata.ino()
                    && self.change_seconds == metadata.ctime()
                    && self.change_nanoseconds == metadata.ctime_nsec()
            }
            #[cfg(not(unix))]
            {
                true
            }
        }
    }
}

impl std::fmt::Debug for FilePart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FilePart")
            .field("path", &"[REDACTED]")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .finish()
    }
}

impl FilePart {
    /// Asynchronously validate a path-backed part without blocking an async
    /// request path on filesystem metadata I/O.
    pub async fn from_path_async(path: &Path) -> ZaiResult<Self> {
        let meta = tokio::fs::symlink_metadata(path)
            .await
            .map_err(ZaiError::from)?;
        Self::from_metadata(path, &meta)
    }

    fn from_metadata(path: &Path, meta: &std::fs::Metadata) -> ZaiResult<Self> {
        validate_metadata(meta)?;
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("filename must be valid UTF-8"))?;
        validate_basename(basename)?;
        Ok(Self {
            path: path.to_path_buf(),
            filename: basename.to_string(),
            content_type: guess_content_type(basename),
            identity: FileIdentity::from_metadata(meta),
        })
    }

    /// Override the multipart filename after validating it as a basename.
    pub fn with_filename(mut self, filename: impl Into<String>) -> ZaiResult<Self> {
        let filename = filename.into();
        validate_basename(&filename)?;
        self.filename = filename;
        Ok(self)
    }

    /// Override the MIME type after validating its header syntax.
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> ZaiResult<Self> {
        let content_type = content_type.into();
        validate_content_type(&content_type)?;
        self.content_type = content_type;
        Ok(self)
    }

    /// Size captured when the local file was validated.
    pub fn len(&self) -> u64 {
        self.identity.len
    }
}

/// A factory that builds a fresh multipart form per attempt.
#[derive(Clone)]
pub struct MultipartBodyFactory {
    parts: Vec<(String, FilePart)>,
    memory_parts: Vec<(String, String, String, bytes::Bytes)>,
    fields: Vec<(String, bytes::Bytes)>,
}

impl std::fmt::Debug for MultipartBodyFactory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MultipartBodyFactory")
            .field("path_parts", &self.parts.len())
            .field("memory_parts", &self.memory_parts.len())
            .field("fields", &self.fields.len())
            .finish()
    }
}

impl MultipartBodyFactory {
    /// Create an empty multipart form factory.
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            memory_parts: Vec::new(),
            fields: Vec::new(),
        }
    }

    /// Add a file under an explicit multipart field name.
    pub fn file_named(mut self, field_name: impl Into<String>, part: FilePart) -> ZaiResult<Self> {
        if self.file_count() >= MULTIPART_MAX_FILE_PARTS {
            return Err(invalid(&format!(
                "multipart exceeds {MULTIPART_MAX_FILE_PARTS} file parts"
            )));
        }
        let field_name = field_name.into();
        validate_field_name(&field_name)?;
        self.parts.push((field_name, part));
        Ok(self)
    }

    /// Add a non-file field, enforcing the aggregate field-byte budget.
    pub fn field(self, name: impl Into<String>, value: impl Into<String>) -> ZaiResult<Self> {
        self.field_with_total_limit(name, value, MULTIPART_FIELD_BYTES_MAX)
    }

    /// Add a text field using a caller-specific aggregate limit.
    ///
    /// This is crate-private for protocols such as ASR whose documented
    /// base64 audio field is larger than the normal metadata budget.
    pub(crate) fn field_with_total_limit(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
        total_limit: u64,
    ) -> ZaiResult<Self> {
        let value = bytes::Bytes::from(value.into());
        self.field_bytes_with_total_limit(name, value, total_limit)
    }

    /// Add a byte-backed text field using a caller-specific aggregate limit.
    ///
    /// Keeping the payload in `Bytes` lets every retry share the immutable
    /// allocation while still constructing a fresh multipart body.
    pub(crate) fn field_bytes_with_total_limit(
        mut self,
        name: impl Into<String>,
        value: impl Into<bytes::Bytes>,
        total_limit: u64,
    ) -> ZaiResult<Self> {
        let name = name.into();
        let value = value.into();
        validate_field_name(&name)?;
        let added = name.len().saturating_add(value.len());
        let current = self.fields.iter().fold(0usize, |total, (key, value)| {
            total.saturating_add(key.len()).saturating_add(value.len())
        });
        if current.saturating_add(added) as u64 > total_limit {
            return Err(payload_too_large(total_limit));
        }
        self.fields.push((name, value));
        Ok(self)
    }

    /// Add an already-buffered file part. This is used by public request
    /// builders that accept bytes rather than filesystem paths.
    pub fn bytes_named(
        mut self,
        field_name: impl Into<String>,
        filename: impl Into<String>,
        content_type: impl Into<String>,
        bytes: impl Into<bytes::Bytes>,
    ) -> ZaiResult<Self> {
        if self.file_count() >= MULTIPART_MAX_FILE_PARTS {
            return Err(invalid(&format!(
                "multipart exceeds {MULTIPART_MAX_FILE_PARTS} file parts"
            )));
        }
        let field_name = field_name.into();
        validate_field_name(&field_name)?;
        let filename = filename.into();
        validate_basename(&filename)?;
        let content_type = content_type.into();
        validate_content_type(&content_type)?;
        let bytes = bytes.into();
        if bytes.len() as u64 > MULTIPART_FILE_BYTES_MAX {
            return Err(payload_too_large(MULTIPART_FILE_BYTES_MAX));
        }
        self.memory_parts
            .push((field_name, filename, content_type, bytes));
        Ok(self)
    }

    fn file_count(&self) -> usize {
        self.parts.len().saturating_add(self.memory_parts.len())
    }

    /// Asynchronously build a fresh replayable form for one attempt.
    ///
    /// Every path-backed file is reopened, revalidated, and streamed up to the
    /// size observed on the opened handle. In-memory parts use cheap `Bytes`
    /// clones.
    pub async fn build(&self) -> ZaiResult<reqwest::multipart::Form> {
        let mut form = reqwest::multipart::Form::new();
        for (name, value) in &self.fields {
            let part =
                reqwest::multipart::Part::stream_with_length(value.clone(), value.len() as u64);
            form = form.part(name.clone(), part);
        }
        for (field_name, part) in &self.parts {
            validate_basename(&part.filename)?;
            validate_content_type(&part.content_type)?;
            let (file, length) = open_streaming_file(&part.path, &part.identity).await?;
            let stream = ReaderStream::new(file.take(length));
            let body = reqwest::Body::wrap_stream(stream);
            let mp = reqwest::multipart::Part::stream_with_length(body, length)
                .file_name(part.filename.clone())
                .mime_str(&part.content_type)
                .map_err(|e| invalid(&format!("invalid mime: {e}")))?;
            form = form.part(field_name.clone(), mp);
        }
        for (field_name, filename, content_type, bytes) in &self.memory_parts {
            let part =
                reqwest::multipart::Part::stream_with_length(bytes.clone(), bytes.len() as u64)
                    .file_name(filename.clone())
                    .mime_str(content_type)
                    .map_err(|e| invalid(&format!("invalid mime: {e}")))?;
            form = form.part(field_name.clone(), part);
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

fn validate_field_name(name: &str) -> ZaiResult<()> {
    if name.is_empty() {
        return Err(invalid("multipart field name must not be empty"));
    }
    if !name
        .bytes()
        .all(|byte| (0x20..=0x7e).contains(&byte) && !matches!(byte, b'"' | b'\\'))
    {
        return Err(invalid(
            "multipart field name must be printable ASCII without quotes or backslashes",
        ));
    }
    Ok(())
}

fn validate_content_type(content_type: &str) -> ZaiResult<()> {
    reqwest::multipart::Part::bytes(Vec::new())
        .mime_str(content_type)
        .map(|_| ())
        .map_err(|error| invalid(&format!("invalid mime: {error}")))
}

fn validate_metadata(metadata: &std::fs::Metadata) -> ZaiResult<()> {
    if metadata.is_symlink() {
        return Err(invalid("multipart file must not be a symlink"));
    }
    if !metadata.is_file() {
        return Err(invalid("multipart file must be a regular file"));
    }
    if metadata.len() > MULTIPART_FILE_BYTES_MAX {
        return Err(payload_too_large(MULTIPART_FILE_BYTES_MAX));
    }
    Ok(())
}

async fn open_streaming_file(
    path: &Path,
    expected: &FileIdentity,
) -> ZaiResult<(tokio::fs::File, u64)> {
    let path_metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(ZaiError::from)?;
    validate_metadata(&path_metadata)?;
    if !expected.matches(&path_metadata) {
        return Err(invalid(
            "multipart file changed after the request was prepared",
        ));
    }

    let file = tokio::fs::File::open(path).await.map_err(ZaiError::from)?;
    let file_metadata = file.metadata().await.map_err(ZaiError::from)?;
    validate_metadata(&file_metadata)?;
    if !expected.matches(&file_metadata) {
        return Err(invalid(
            "multipart file changed after the request was prepared",
        ));
    }

    // Detect replacement between the no-symlink check and open on Unix. The
    // stream reads from the already-opened inode, so later path replacement
    // cannot redirect an in-flight upload.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if path_metadata.dev() != file_metadata.dev() || path_metadata.ino() != file_metadata.ino()
        {
            return Err(invalid("multipart file changed while it was opened"));
        }
    }

    Ok((file, file_metadata.len()))
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
    use futures_util::StreamExt;

    use super::*;

    async fn normalized_wire(form: reqwest::multipart::Form) -> Vec<u8> {
        let boundary = form.boundary().as_bytes().to_vec();
        let mut stream = Box::pin(form.into_stream());
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            body.extend_from_slice(&chunk.unwrap());
        }

        String::from_utf8(body)
            .unwrap()
            .replace(std::str::from_utf8(&boundary).unwrap(), "TEST_BOUNDARY")
            .into_bytes()
    }

    #[tokio::test]
    async fn rejects_symlink_and_nonregular() {
        let dir = tempfile::tempdir().unwrap();
        // symlink (on platforms that support it)
        let real = dir.path().join("real.txt");
        std::fs::write(&real, b"hi").unwrap();
        let _link = dir.path().join("link.txt");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real, &_link).unwrap();
            assert!(FilePart::from_path_async(&_link).await.is_err());
        }
        // directory is not regular
        assert!(FilePart::from_path_async(dir.path()).await.is_err());
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

    #[tokio::test]
    async fn part_count_limit_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let mut factory = MultipartBodyFactory::new();
        for i in 0..MULTIPART_MAX_FILE_PARTS {
            let p = dir.path().join(format!("f{i}.txt"));
            std::fs::write(&p, b"x").unwrap();
            factory = factory
                .file_named("file", FilePart::from_path_async(&p).await.unwrap())
                .unwrap();
        }
        // The 17th part is refused.
        let extra = dir.path().join("f17.txt");
        std::fs::write(&extra, b"x").unwrap();
        assert!(
            factory
                .file_named("file", FilePart::from_path_async(&extra).await.unwrap())
                .is_err()
        );
    }

    #[tokio::test]
    async fn mixed_file_kinds_share_one_part_limit() {
        let mut factory = MultipartBodyFactory::new();
        for index in 0..MULTIPART_MAX_FILE_PARTS {
            factory = factory
                .bytes_named(
                    "file",
                    format!("f{index}.txt"),
                    "text/plain",
                    b"x".as_slice(),
                )
                .unwrap();
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extra.txt");
        std::fs::write(&path, b"x").unwrap();
        assert!(
            factory
                .file_named("file", FilePart::from_path_async(&path).await.unwrap())
                .is_err()
        );
    }

    #[test]
    fn field_metadata_is_validated_before_build() {
        assert!(MultipartBodyFactory::new().field("bad\nname", "x").is_err());
        assert!(
            MultipartBodyFactory::new()
                .bytes_named("file", "a.txt", "not a mime", b"x".as_slice())
                .is_err()
        );

        let large_value = "x".repeat(MULTIPART_FIELD_BYTES_MAX as usize);
        assert!(
            MultipartBodyFactory::new()
                .field("audio", large_value.clone())
                .is_err()
        );
        assert!(
            MultipartBodyFactory::new()
                .field_with_total_limit("audio", large_value, MULTIPART_FIELD_BYTES_MAX + 16,)
                .is_ok()
        );
    }

    #[test]
    fn byte_backed_fields_enforce_the_aggregate_limit() {
        let factory = MultipartBodyFactory::new()
            .field_bytes_with_total_limit("a", bytes::Bytes::from_static(b"1234"), 10)
            .unwrap()
            .field_bytes_with_total_limit("bb", bytes::Bytes::from_static(b"567"), 10)
            .unwrap();

        assert_eq!(factory.fields[0].1, bytes::Bytes::from_static(b"1234"));
        assert_eq!(factory.fields[1].1, bytes::Bytes::from_static(b"567"));
        assert!(
            factory
                .clone()
                .field_bytes_with_total_limit("c", bytes::Bytes::new(), 10)
                .is_err()
        );
        assert!(factory.field_with_total_limit("c", "", 10).is_err());
    }

    #[tokio::test]
    async fn byte_backed_fields_rebuild_identically_for_retries() {
        let factory = MultipartBodyFactory::new()
            .field("first", "one")
            .unwrap()
            .field_bytes_with_total_limit(
                "second",
                bytes::Bytes::from_static(b"two"),
                MULTIPART_FIELD_BYTES_MAX,
            )
            .unwrap();

        let first_attempt = normalized_wire(factory.build().await.unwrap()).await;
        let retry_attempt = normalized_wire(factory.build().await.unwrap()).await;

        assert_eq!(first_attempt, retry_attempt);
    }

    #[tokio::test]
    async fn byte_backed_text_fields_preserve_wire_metadata_and_order() {
        let actual = MultipartBodyFactory::new()
            .field("first", "hello")
            .unwrap()
            .field_bytes_with_total_limit(
                "second",
                bytes::Bytes::from_static("世界".as_bytes()),
                MULTIPART_FIELD_BYTES_MAX,
            )
            .unwrap()
            .build()
            .await
            .unwrap();
        let legacy = reqwest::multipart::Form::new()
            .text("first", "hello")
            .text("second", "世界");

        let actual = normalized_wire(actual).await;
        let legacy = normalized_wire(legacy).await;

        assert_eq!(actual, legacy);
        let wire = String::from_utf8(actual).unwrap();
        let first = wire.find("name=\"first\"").unwrap();
        let second = wire.find("name=\"second\"").unwrap();
        assert!(first < second);
    }

    #[test]
    fn content_type_guessing() {
        assert_eq!(guess_content_type("a.png"), "image/png");
        assert_eq!(guess_content_type("a.WAV"), "audio/wav");
        assert_eq!(guess_content_type("a.unknown"), "application/octet-stream");
    }

    #[tokio::test]
    async fn prepared_path_part_rejects_later_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("document.txt");
        std::fs::write(&path, b"original").unwrap();
        let part = FilePart::from_path_async(&path).await.unwrap();
        let factory = MultipartBodyFactory::new()
            .file_named("file", part)
            .unwrap();

        std::fs::write(&path, b"replacement with a different length").unwrap();
        assert!(factory.build().await.is_err());
    }
}
