//! File attachment support for embedding files in agent prompts.
//!
//! Since upstream agent CLIs only accept text prompts, file attachments are
//! embedded directly in the prompt using an XML envelope. Text files (≤50 KB)
//! are inlined verbatim; binary files and large text files are included as
//! metadata references with `@path` so the agent can use its own tools to
//! access them.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Maximum file size allowed for attachments (10 MB).
const MAX_ATTACHMENT_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum size for inline text content (50 KB). Text files larger than this
/// are included as references instead of being inlined.
const MAX_INLINE_SIZE: u64 = 50 * 1024;

/// The content of a file attachment.
#[derive(Debug)]
pub enum AttachmentContent {
    /// Text file content inlined verbatim (≤50 KB).
    Text(String),
    /// Binary file or large text file — metadata only, no content.
    Reference,
}

/// A resolved file attachment ready for embedding in a prompt.
#[derive(Debug)]
pub struct Attachment {
    pub path: PathBuf,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
    pub content: AttachmentContent,
}

impl Attachment {
    /// Load an attachment from a file path.
    ///
    /// The file must exist and be ≤10 MB. Text files ≤50 KB are read into
    /// memory; everything else becomes a reference.
    pub fn from_path(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .with_context(|| format!("File not found: {}", path.display()))?;

        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("Cannot read file metadata: {}", path.display()))?;

        let size = metadata.len();
        if size > MAX_ATTACHMENT_SIZE {
            bail!(
                "File too large: {} ({} bytes, max {} bytes)",
                path.display(),
                size,
                MAX_ATTACHMENT_SIZE
            );
        }

        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let mime_type = mime_from_extension(&ext).to_string();
        let is_text = is_text_mime(&mime_type);

        let content = if is_text && size <= MAX_INLINE_SIZE {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read text file: {}", path.display()))?;
            AttachmentContent::Text(text)
        } else {
            AttachmentContent::Reference
        };

        Ok(Self {
            path,
            filename,
            mime_type,
            size,
            content,
        })
    }
}

/// Format attachments as an XML prefix to prepend to a prompt.
pub fn format_attachments_prefix(attachments: &[Attachment]) -> String {
    let mut out = String::from("<attached-files>\n");
    for att in attachments {
        match &att.content {
            AttachmentContent::Text(text) => {
                out.push_str(&format!(
                    "<file name=\"{}\" path=\"{}\" mime=\"{}\" size=\"{}\" encoding=\"utf-8\">\n{}\n</file>\n",
                    att.filename,
                    att.path.display(),
                    att.mime_type,
                    att.size,
                    text,
                ));
            }
            AttachmentContent::Reference => {
                let encoding = if is_text_mime(&att.mime_type) {
                    "utf-8"
                } else {
                    "binary"
                };
                out.push_str(&format!(
                    "<file name=\"{}\" path=\"{}\" mime=\"{}\" size=\"{}\" encoding=\"{}\">\n\
                     (content not included, use @{} to access this file)\n</file>\n",
                    att.filename,
                    att.path.display(),
                    att.mime_type,
                    att.size,
                    encoding,
                    att.path.display(),
                ));
            }
        }
    }
    out.push_str("</attached-files>\n\n");
    out
}

/// Look up a MIME type from a file extension.
fn mime_from_extension(ext: &str) -> &'static str {
    match ext {
        // Text
        "txt" | "text" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "csv" => "text/csv",
        "xml" => "text/xml",
        "svg" => "image/svg+xml",
        // Code
        "rs" => "text/x-rust",
        "py" => "text/x-python",
        "js" | "mjs" | "cjs" => "text/javascript",
        "ts" | "mts" | "cts" => "text/typescript",
        "tsx" | "jsx" => "text/typescript",
        "java" => "text/x-java",
        "kt" | "kts" => "text/x-kotlin",
        "swift" => "text/x-swift",
        "cs" => "text/x-csharp",
        "go" => "text/x-go",
        "rb" => "text/x-ruby",
        "php" => "text/x-php",
        "c" | "h" => "text/x-c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "text/x-c++",
        "sh" | "bash" | "zsh" | "fish" => "text/x-shellscript",
        "sql" => "text/x-sql",
        "r" => "text/x-r",
        "lua" => "text/x-lua",
        "pl" | "pm" => "text/x-perl",
        "scala" => "text/x-scala",
        "zig" => "text/x-zig",
        "hs" => "text/x-haskell",
        "ex" | "exs" => "text/x-elixir",
        "dart" => "text/x-dart",
        "v" | "sv" => "text/x-verilog",
        "vhd" | "vhdl" => "text/x-vhdl",
        // Config/data
        "json" => "application/json",
        "jsonl" | "ndjson" => "application/x-ndjson",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "ini" | "cfg" | "conf" => "text/plain",
        "env" => "text/plain",
        "lock" => "text/plain",
        "log" => "text/plain",
        // Documents
        "pdf" => "application/pdf",
        "doc" | "docx" => "application/msword",
        "xls" | "xlsx" => "application/vnd.ms-excel",
        "ppt" | "pptx" => "application/vnd.ms-powerpoint",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tiff" | "tif" => "image/tiff",
        // Audio/Video
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        // Archives
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "bz2" => "application/x-bzip2",
        "xz" => "application/x-xz",
        "7z" => "application/x-7z-compressed",
        // Binary
        "wasm" => "application/wasm",
        "exe" | "dll" | "so" | "dylib" => "application/octet-stream",
        // Makefile, Dockerfile, etc. — no extension
        _ => "application/octet-stream",
    }
}

/// Returns `true` if the MIME type represents text content that can be inlined.
fn is_text_mime(mime: &str) -> bool {
    mime.starts_with("text/")
        || mime == "application/json"
        || mime == "application/x-ndjson"
        || mime == "application/yaml"
        || mime == "application/toml"
        || mime == "application/xml"
        || mime == "image/svg+xml"
}

#[cfg(test)]
#[path = "attachment_tests.rs"]
mod tests;
