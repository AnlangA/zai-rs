//! 统一的文件I/O操作
//!
//! 提供异步文件读取、MIME类型推断等功能

use std::path::Path;

use tokio::fs;

/// 文件内容封装
#[derive(Debug)]
pub struct FileContent {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub mime_type: String,
    pub size: u64,
}

/// 根据文件扩展名推断MIME类型
pub fn infer_mime_type(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        // 图片
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("bmp") => "image/bmp",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",

        // 音频
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",

        // 文档
        Some("pdf") => "application/pdf",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("ppt") => "application/vnd.ms-powerpoint",
        Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        Some("txt") => "text/plain",
        Some("md") => "text/markdown",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("xml") => "application/xml",

        // 视频
        Some("mp4") => "video/mp4",
        Some("avi") => "video/x-msvideo",
        Some("mkv") => "video/x-matroska",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",

        // 压缩文件
        Some("zip") => "application/zip",
        Some("rar") => "application/vnd.rar",
        Some("7z") => "application/x-7z-compressed",
        Some("tar") => "application/x-tar",
        Some("gz") => "application/gzip",

        _ => "application/octet-stream",
    }
    .to_string()
}

/// 异步读取文件内容
///
/// 返回包含文件字节、文件名、MIME类型和大小的结构体
pub async fn read_file(path: impl AsRef<Path>) -> crate::ZaiResult<FileContent> {
    let path = path.as_ref();

    // 1. 检查文件存在
    if !path.exists() {
        return Err(crate::client::error::ZaiError::FileError {
            code: 0,
            message: format!("File not found: {}", path.display()),
        });
    }

    // 2. 异步读取
    let bytes = fs::read(path).await?;

    // 3. 提取元数据
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mime_type = infer_mime_type(path);
    let size = bytes.len() as u64;

    Ok(FileContent {
        bytes,
        file_name,
        mime_type,
        size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_mime_type_images() {
        assert_eq!(infer_mime_type(Path::new("test.png")), "image/png");
        assert_eq!(infer_mime_type(Path::new("test.jpg")), "image/jpeg");
        assert_eq!(infer_mime_type(Path::new("test.jpeg")), "image/jpeg");
        assert_eq!(infer_mime_type(Path::new("test.bmp")), "image/bmp");
        assert_eq!(infer_mime_type(Path::new("test.gif")), "image/gif");
        assert_eq!(infer_mime_type(Path::new("test.webp")), "image/webp");
    }

    #[test]
    fn test_infer_mime_type_audio() {
        assert_eq!(infer_mime_type(Path::new("test.mp3")), "audio/mpeg");
        assert_eq!(infer_mime_type(Path::new("test.wav")), "audio/wav");
        assert_eq!(infer_mime_type(Path::new("test.ogg")), "audio/ogg");
        assert_eq!(infer_mime_type(Path::new("test.m4a")), "audio/mp4");
        assert_eq!(infer_mime_type(Path::new("test.aac")), "audio/aac");
    }

    #[test]
    fn test_infer_mime_type_documents() {
        assert_eq!(infer_mime_type(Path::new("test.pdf")), "application/pdf");
        assert_eq!(infer_mime_type(Path::new("test.txt")), "text/plain");
        assert_eq!(infer_mime_type(Path::new("test.md")), "text/markdown");
        assert_eq!(infer_mime_type(Path::new("test.json")), "application/json");
        assert_eq!(infer_mime_type(Path::new("test.csv")), "text/csv");
    }

    #[test]
    fn test_infer_mime_type_unknown() {
        assert_eq!(
            infer_mime_type(Path::new("test.unknown")),
            "application/octet-stream"
        );
        assert_eq!(
            infer_mime_type(Path::new("test")),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn test_read_file_success() {
        // Create a temporary test file
        let test_file = "/tmp/test_read_file.txt";
        tokio::fs::write(test_file, b"Hello, World!").await.unwrap();

        let content = read_file(test_file).await.unwrap();

        assert_eq!(content.bytes, b"Hello, World!");
        assert_eq!(content.file_name, "test_read_file.txt");
        assert_eq!(content.mime_type, "text/plain");
        assert_eq!(content.size, 13);

        // Cleanup
        tokio::fs::remove_file(test_file).await.unwrap();
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let result = read_file("/nonexistent/file.txt").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::client::error::ZaiError::FileError { .. } => {},
            _ => panic!("Expected FileError"),
        }
    }
}
