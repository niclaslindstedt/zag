use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_mime_from_extension() {
    assert_eq!(mime_from_extension("rs"), "text/x-rust");
    assert_eq!(mime_from_extension("py"), "text/x-python");
    assert_eq!(mime_from_extension("js"), "text/javascript");
    assert_eq!(mime_from_extension("json"), "application/json");
    assert_eq!(mime_from_extension("toml"), "application/toml");
    assert_eq!(mime_from_extension("png"), "image/png");
    assert_eq!(mime_from_extension("pdf"), "application/pdf");
    assert_eq!(mime_from_extension("txt"), "text/plain");
    assert_eq!(mime_from_extension("unknown"), "application/octet-stream");
}

#[test]
fn test_is_text_mime() {
    assert!(is_text_mime("text/plain"));
    assert!(is_text_mime("text/x-rust"));
    assert!(is_text_mime("text/markdown"));
    assert!(is_text_mime("application/json"));
    assert!(is_text_mime("application/yaml"));
    assert!(is_text_mime("application/toml"));
    assert!(is_text_mime("image/svg+xml"));

    assert!(!is_text_mime("image/png"));
    assert!(!is_text_mime("application/pdf"));
    assert!(!is_text_mime("application/octet-stream"));
    assert!(!is_text_mime("audio/mpeg"));
}

#[test]
fn test_attachment_from_text_file() {
    let mut f = NamedTempFile::with_suffix(".txt").unwrap();
    write!(f, "hello world").unwrap();
    f.flush().unwrap();

    let att = Attachment::from_path(f.path()).unwrap();
    assert_eq!(
        att.filename,
        f.path().file_name().unwrap().to_str().unwrap()
    );
    assert_eq!(att.mime_type, "text/plain");
    assert_eq!(att.size, 11);
    assert!(matches!(att.content, AttachmentContent::Text(ref s) if s == "hello world"));
}

#[test]
fn test_attachment_from_large_text_file() {
    let mut f = NamedTempFile::with_suffix(".txt").unwrap();
    // Write >50 KB of text
    let big = "x".repeat(51 * 1024);
    write!(f, "{}", big).unwrap();
    f.flush().unwrap();

    let att = Attachment::from_path(f.path()).unwrap();
    assert_eq!(att.mime_type, "text/plain");
    assert!(att.size > 50 * 1024);
    assert!(matches!(att.content, AttachmentContent::Reference));
}

#[test]
fn test_attachment_from_binary_file() {
    let mut f = NamedTempFile::with_suffix(".png").unwrap();
    // Write PNG header bytes
    f.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        .unwrap();
    f.flush().unwrap();

    let att = Attachment::from_path(f.path()).unwrap();
    assert_eq!(att.mime_type, "image/png");
    assert!(matches!(att.content, AttachmentContent::Reference));
}

#[test]
fn test_format_attachments_text_inline() {
    let att = Attachment {
        path: PathBuf::from("/tmp/test.txt"),
        filename: "test.txt".to_string(),
        mime_type: "text/plain".to_string(),
        size: 11,
        content: AttachmentContent::Text("hello world".to_string()),
    };

    let prefix = format_attachments_prefix(&[att]);
    assert!(prefix.contains("<attached-files>"));
    assert!(prefix.contains("</attached-files>"));
    assert!(prefix.contains("encoding=\"utf-8\""));
    assert!(prefix.contains("hello world"));
    assert!(prefix.contains("name=\"test.txt\""));
}

#[test]
fn test_format_attachments_reference() {
    let att = Attachment {
        path: PathBuf::from("/tmp/image.png"),
        filename: "image.png".to_string(),
        mime_type: "image/png".to_string(),
        size: 45231,
        content: AttachmentContent::Reference,
    };

    let prefix = format_attachments_prefix(&[att]);
    assert!(prefix.contains("encoding=\"binary\""));
    assert!(prefix.contains("use @/tmp/image.png to access"));
    assert!(!prefix.contains("hello world"));
}

#[test]
fn test_format_attachments_mixed() {
    let text_att = Attachment {
        path: PathBuf::from("/tmp/config.toml"),
        filename: "config.toml".to_string(),
        mime_type: "application/toml".to_string(),
        size: 20,
        content: AttachmentContent::Text("[package]\nname = \"x\"".to_string()),
    };
    let bin_att = Attachment {
        path: PathBuf::from("/tmp/photo.jpg"),
        filename: "photo.jpg".to_string(),
        mime_type: "image/jpeg".to_string(),
        size: 100_000,
        content: AttachmentContent::Reference,
    };

    let prefix = format_attachments_prefix(&[text_att, bin_att]);
    assert!(prefix.contains("config.toml"));
    assert!(prefix.contains("[package]"));
    assert!(prefix.contains("photo.jpg"));
    assert!(prefix.contains("use @/tmp/photo.jpg to access"));
}

#[test]
fn test_format_attachments_large_text_reference() {
    let att = Attachment {
        path: PathBuf::from("/tmp/big.log"),
        filename: "big.log".to_string(),
        mime_type: "text/plain".to_string(),
        size: 60_000,
        content: AttachmentContent::Reference,
    };

    let prefix = format_attachments_prefix(&[att]);
    assert!(prefix.contains("encoding=\"utf-8\""));
    assert!(prefix.contains("use @/tmp/big.log to access"));
}

#[test]
fn test_attachment_file_not_found() {
    let result = Attachment::from_path(Path::new("/nonexistent/file.txt"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("File not found"));
}

#[test]
fn test_attachment_size_limit() {
    // We can't easily create a >10MB temp file in a test, but we can verify
    // the constant is correct.
    assert_eq!(MAX_ATTACHMENT_SIZE, 10 * 1024 * 1024);
    assert_eq!(MAX_INLINE_SIZE, 50 * 1024);
}
