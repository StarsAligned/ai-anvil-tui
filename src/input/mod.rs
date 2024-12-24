pub mod file_system;
pub mod github;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TextSourceError {
    #[error("Invalid source path or URL")]
    InvalidSource,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("GitHub API error: {0}")]
    GitHubError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Path not found: {0}")]
    PathNotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("GitHub rate limit exceeded")]
    RateLimitExceeded,
    #[error("GitHub repository not found")]
    RepoNotFound,
    #[error("File is not valid UTF-8 text: {0}")]
    NotTextFile(String),
}

#[async_trait]
pub trait TextSource: Send + Sync {
    async fn get_file_index(
        &self,
        filter: &FilterConfig,
    ) -> Result<Vec<SourceFile>, TextSourceError>;
    async fn get_file_content(&self, source_file: &SourceFile) -> Result<String, TextSourceError>;
}

#[derive(Debug, Clone)]
pub struct FilterConfig {
    pub additional_text_extensions: HashSet<String>,
    pub additional_binary_extensions: HashSet<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            additional_text_extensions: HashSet::new(),
            additional_binary_extensions: HashSet::new(),
        }
    }
}
impl FilterConfig {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_text_extension(&self, ext: String) -> bool {
        if self.additional_binary_extensions.contains(&ext) {
            return false;
        }
        if self.additional_text_extensions.contains(&ext) {
            return true;
        }
        !NON_TEXT_EXTENSIONS.contains(ext.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub source_type: SourceType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceType {
    FileSystem {
        base_path: std::path::PathBuf,
    },
    GitHub {
        owner: String,
        repo: String,
        branch: String,
    },
}

static NON_TEXT_EXTENSIONS: Lazy<HashSet<&str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.extend([
        "exe", "dll", "so", "dylib", "bin", "app", "msi", "sys", "com", "o", "obj", "class",
    ]);
    set.extend([
        "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "img", "tgz",
    ]);
    set.extend([
        "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "ico", "svg", "eps", "raw", "cr2",
        "nef", "heic",
    ]);
    set.extend([
        "mp3", "wav", "ogg", "flac", "m4a", "wma", "aac", "mid", "midi", "aiff",
    ]);
    set.extend([
        "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp",
    ]);
    set.extend([
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pages", "numbers", "key", "indd",
        "psd", "ai",
    ]);
    set.extend(["db", "sqlite", "mdb", "accdb", "dbf", "dat", "mdf", "sdf"]);
    set.extend(["ttf", "otf", "woff", "woff2", "eot"]);
    set.extend([
        "pyc", "pyo", "pyd", "jar", "war", "deb", "rpm", "lib", "a", "pak", "cache", "idx", "mo",
        "gmo", "pdb",
    ]);
    set
});

pub async fn create_text_source(source: &str) -> Result<Box<dyn TextSource>, TextSourceError> {
    if source.starts_with("https://github.com") {
        let (owner, repo, branch, subpath) = github::GitHubSource::parse_github_url(source)?;
        Ok(Box::new(github::GitHubSource::new(
            owner, repo, branch, subpath,
        )))
    } else {
        Ok(Box::new(file_system::FileSystemSource::new(source)?))
    }
}
