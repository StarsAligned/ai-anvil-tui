use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashSet;
use thiserror::Error;
use url::Url;
use reqwest;
use serde::Deserialize;
use async_trait::async_trait;
use once_cell::sync::Lazy;

// Define a set of non-text file extensions
static NON_TEXT_EXTENSIONS: Lazy<HashSet<&str>> = Lazy::new(|| {
	let mut set = HashSet::new();
	// Binary and Executable
	set.extend([
		"exe", "dll", "so", "dylib", "bin", "app",
		"msi", "sys", "com", "o", "obj", "class",
	]);
	// Archives and Compressed
	set.extend([
		"zip", "rar", "7z", "tar", "gz", "bz2", "xz",
		"iso", "dmg", "img", "tgz",
	]);
	// Images
	set.extend([
		"jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp",
		"ico", "svg", "eps", "raw", "cr2", "nef", "heic",
	]);
	// Audio
	set.extend([
		"mp3", "wav", "ogg", "flac", "m4a", "wma", "aac",
		"mid", "midi", "aiff",
	]);
	// Video
	set.extend([
		"mp4", "avi", "mkv", "mov", "wmv", "flv", "webm",
		"m4v", "mpg", "mpeg", "3gp",
	]);
	// Documents and Publishing
	set.extend([
		"pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
		"pages", "numbers", "key", "indd", "psd", "ai",
	]);
	// Database and Data
	set.extend([
		"db", "sqlite", "mdb", "accdb", "dbf", "dat",
		"mdf", "sdf",
	]);
	// Font files
	set.extend([
		"ttf", "otf", "woff", "woff2", "eot",
	]);
	// Other binary formats
	set.extend([
		"pyc", "pyo", "pyd", // Python bytecode
		"jar", "war", // Java archives
		"deb", "rpm", // Linux packages
		"lib", "a", // Static libraries
		"pak", "cache", // Various cache/package formats
		"idx", "bin", // Binary index/data files
		"mo", "gmo", // Gettext binary translations
		"pdb", // Debug symbols
	]);
	set
});

#[derive(Debug, Clone)]
pub struct FilterConfig {
	pub show_hidden: bool,
	pub additional_text_extensions: HashSet<String>,
	pub additional_binary_extensions: HashSet<String>,
}

impl Default for FilterConfig {
	fn default() -> Self {
		Self {
			show_hidden: false,
			additional_text_extensions: HashSet::new(),
			additional_binary_extensions: HashSet::new(),
		}
	}
}

impl FilterConfig {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_show_hidden(mut self, show_hidden: bool) -> Self {
		self.show_hidden = show_hidden;
		self
	}

	fn is_text_extension(&self, ext: String) -> bool {
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
	FileSystem { base_path: PathBuf },
	GitHub { owner: String, repo: String, branch: String },
}

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
	async fn get_file_index(&self, filter: &FilterConfig) -> Result<Vec<SourceFile>, TextSourceError>;
	async fn get_file_content(&self, source_file: &SourceFile) -> Result<String, TextSourceError>;
}

pub struct FileSystemSource {
	base_path: PathBuf,
}

pub struct GitHubSource {
	owner: String,
	repo: String,
	branch: String,
	subpath: Option<String>,
	client: reqwest::Client,
}

#[derive(Deserialize)]
struct GitHubTreeResponse {
	tree: Vec<GitHubContent>,
}

#[derive(Deserialize)]
struct GitHubContent {
	path: String,
	r#type: String,
}

fn get_extension(path: &str) -> Option<String> {
	Path::new(path)
		.extension()
		.and_then(|ext| ext.to_str())
		.map(|ext| ext.to_lowercase())
}

fn should_include_path(path: &str, filter: &FilterConfig) -> bool {
	// Always filter out backup files (ending with ~)
	if path.ends_with('~') {
		return false;
	}

	// Get file extension and check if it's a text file
	if let Some(ext) = get_extension(path) {
		if !filter.is_text_extension(ext) {
			return false;
		}
	}

	// Split path into components
	let components: Vec<&str> = path.split(std::path::MAIN_SEPARATOR)
		.filter(|s| !s.is_empty())
		.collect();

	// If show_hidden is false, exclude any path that contains a dot-prefixed component
	if !filter.show_hidden {
		for component in &components {
			if component.starts_with('.') {
				return false;
			}
		}
	}

	true
}

impl FileSystemSource {
	pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, TextSourceError> {
		let base_path = path.as_ref().to_path_buf();
		if !base_path.exists() {
			return Err(TextSourceError::PathNotFound(
				base_path.to_string_lossy().to_string()
			));
		}
		if !base_path.is_dir() {
			return Err(TextSourceError::InvalidSource);
		}

		match fs::read_dir(&base_path) {
			Ok(_) => Ok(Self { base_path }),
			Err(_) => Err(TextSourceError::PermissionDenied(
				base_path.to_string_lossy().to_string()
			)),
		}
	}

	fn collect_files(&self, dir: &Path, files: &mut Vec<SourceFile>, filter: &FilterConfig) -> Result<(), TextSourceError> {
		let entries = fs::read_dir(dir).map_err(|e| {
			if e.kind() == std::io::ErrorKind::PermissionDenied {
				TextSourceError::PermissionDenied(dir.to_string_lossy().to_string())
			} else {
				TextSourceError::IoError(e)
			}
		})?;

		for entry in entries {
			let entry = entry.map_err(TextSourceError::IoError)?;
			let path = entry.path();

			// Get relative path for filtering
			let relative_path = path.strip_prefix(&self.base_path)
				.map_err(|_| TextSourceError::InvalidSource)?
				.to_string_lossy()
				.into_owned();

			// Check if this path should be included
			if !should_include_path(&relative_path, filter) {
				continue;
			}

			if path.is_file() {
				files.push(SourceFile {
					path: relative_path,
					source_type: SourceType::FileSystem {
						base_path: self.base_path.clone(),
					},
				});
			} else if path.is_dir() {
				// Only recurse into directories that pass the filter
				self.collect_files(&path, files, filter)?;
			}
		}
		Ok(())
	}
}

impl GitHubSource {
	pub fn new(owner: String, repo: String, branch: String, subpath: Option<String>) -> Self {
		let client = reqwest::Client::builder()
			.user_agent("rust-text-source")
			.build()
			.unwrap_or_default();

		Self {
			owner,
			repo,
			branch,
			subpath,
			client,
		}
	}

	fn parse_github_url(url: &str) -> Result<(String, String, String, Option<String>), TextSourceError> {
		let url = Url::parse(url).map_err(|_| TextSourceError::InvalidSource)?;

		if url.scheme() != "https" || url.host_str() != Some("github.com") {
			return Err(TextSourceError::InvalidSource);
		}

		let segments: Vec<&str> = url.path_segments()
			.ok_or(TextSourceError::InvalidSource)?
			.collect();

		if segments.len() < 2 {
			return Err(TextSourceError::InvalidSource);
		}

		let owner = segments[0].to_string();
		let repo = segments[1].trim_end_matches(".git").to_string();

		let mut branch = String::from("main");
		let mut subpath = None;

		let remaining_path: Vec<&str> = segments[2..].to_vec();
		if remaining_path.len() >= 2 && remaining_path[0] == "tree" {
			branch = remaining_path[1].to_string();
			if remaining_path.len() > 2 {
				subpath = Some(remaining_path[2..].join("/"));
			}
		}

		Ok((owner, repo, branch, subpath))
	}

	async fn handle_github_response<T: for<'de> serde::Deserialize<'de>>(
		&self,
		response: reqwest::Response
	) -> Result<T, TextSourceError> {
		match response.status() {
			status if status.is_success() => {
				response.json::<T>().await.map_err(TextSourceError::NetworkError)
			}
			status if status.as_u16() == 403 => {
				Err(TextSourceError::RateLimitExceeded)
			}
			status if status.as_u16() == 404 => {
				Err(TextSourceError::RepoNotFound)
			}
			_ => {
				let error_text = response.text().await
					.map_err(TextSourceError::NetworkError)?;
				Err(TextSourceError::GitHubError(error_text))
			}
		}
	}
}

#[async_trait]
impl TextSource for FileSystemSource {
	async fn get_file_index(&self, filter: &FilterConfig) -> Result<Vec<SourceFile>, TextSourceError> {
		let mut files = Vec::new();
		self.collect_files(&self.base_path, &mut files, filter)?;
		Ok(files)
	}

	async fn get_file_content(&self, source_file: &SourceFile) -> Result<String, TextSourceError> {
		match &source_file.source_type {
			SourceType::FileSystem { base_path } => {
				let full_path = base_path.join(&source_file.path);
				let path_for_error = full_path.clone();

				if !full_path.exists() {
					return Err(TextSourceError::PathNotFound(
						path_for_error.to_string_lossy().to_string()
					));
				}

				// First read the file as bytes
				let bytes = fs::read(&full_path).map_err(|e| {
					if e.kind() == std::io::ErrorKind::PermissionDenied {
						TextSourceError::PermissionDenied(
							path_for_error.to_string_lossy().to_string()
						)
					} else {
						TextSourceError::IoError(e)
					}
				})?;

				// Then try to convert to UTF-8
				String::from_utf8(bytes).map_err(|_| {
					TextSourceError::NotTextFile(path_for_error.to_string_lossy().to_string())
				})
			}
			_ => Err(TextSourceError::InvalidSource),
		}
	}
}

#[async_trait]
impl TextSource for GitHubSource {
	async fn get_file_index(&self, filter: &FilterConfig) -> Result<Vec<SourceFile>, TextSourceError> {
		let url = format!(
			"https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
			self.owner, self.repo, self.branch
		);

		let response = self.client
			.get(&url)
			.send()
			.await
			.map_err(TextSourceError::NetworkError)?;

		let tree_response: GitHubTreeResponse = self.handle_github_response(response).await?;

		let filtered_contents = tree_response.tree
			.into_iter()
			.filter(|item| item.r#type == "blob")
			.filter(|item| {
				// Apply subpath filter first
				if let Some(ref subpath) = self.subpath {
					if !item.path.starts_with(subpath) {
						return false;
					}
				}

				// Get the relative path after subpath
				let relative_path = if let Some(ref subpath) = self.subpath {
					if let Some(stripped) = item.path.strip_prefix(subpath) {
						stripped.trim_start_matches('/')
					} else {
						&item.path
					}
				} else {
					&item.path
				};

				// Apply hidden files and binary files filter
				should_include_path(relative_path, filter)
			})
			.map(|item| {
				let path = if let Some(ref subpath) = self.subpath {
					item.path.strip_prefix(subpath)
						.unwrap_or(&item.path)
						.trim_start_matches('/')
						.to_string()
				} else {
					item.path
				};

				SourceFile {
					path,
					source_type: SourceType::GitHub {
						owner: self.owner.clone(),
						repo: self.repo.clone(),
						branch: self.branch.clone(),
					},
				}
			})
			.collect();

		Ok(filtered_contents)
	}

	async fn get_file_content(&self, source_file: &SourceFile) -> Result<String, TextSourceError> {
		match &source_file.source_type {
			SourceType::GitHub { owner, repo, branch } => {
				let file_path = if let Some(ref subpath) = self.subpath {
					format!("{}/{}", subpath, source_file.path)
				} else {
					source_file.path.clone()
				};

				let url = format!(
					"https://raw.githubusercontent.com/{}/{}/{}/{}",
					owner, repo, branch, file_path
				);

				let response = self.client
					.get(&url)
					.send()
					.await
					.map_err(TextSourceError::NetworkError)?;

				match response.status() {
					status if status.is_success() => {
						// Get the response as bytes first
						let bytes = response.bytes().await
							.map_err(TextSourceError::NetworkError)?;

						// Try to convert to UTF-8
						String::from_utf8(bytes.to_vec()).map_err(|_| {
							TextSourceError::NotTextFile(file_path)
						})
					}
					status if status.as_u16() == 404 => {
						Err(TextSourceError::PathNotFound(source_file.path.clone()))
					}
					status if status.as_u16() == 403 => {
						Err(TextSourceError::RateLimitExceeded)
					}
					_ => {
						Err(TextSourceError::GitHubError(
							response.status().to_string()
						))
					}
				}
			}
			_ => Err(TextSourceError::InvalidSource),
		}
	}
}

pub async fn create_text_source(source: &str) -> Result<Box<dyn TextSource>, TextSourceError> {
	if source.starts_with("https://github.com") {
		let (owner, repo, branch, subpath) = GitHubSource::parse_github_url(source)?;
		Ok(Box::new(GitHubSource::new(owner, repo, branch, subpath)))
	} else {
		Ok(Box::new(FileSystemSource::new(source)?))
	}
}