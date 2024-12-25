use super::{FilterConfig, SourceFile, TextSource, TextSourceError};
use async_trait::async_trait;
use reqwest;
use url::Url;

pub struct GitHubSource {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub subpath: Option<String>,
    pub client: reqwest::Client,
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
    pub fn parse_github_url(
        url: &str,
    ) -> Result<(String, String, String, Option<String>), TextSourceError> {
        let parsed = Url::parse(url).map_err(|_| TextSourceError::InvalidSource)?;
        if parsed.scheme() != "https" || parsed.host_str() != Some("github.com") {
            return Err(TextSourceError::InvalidSource);
        }
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|seg| seg.collect())
            .ok_or(TextSourceError::InvalidSource)?;
        if segments.len() < 2 {
            return Err(TextSourceError::InvalidSource);
        }
        let owner = segments[0].to_string();
        let repository = segments[1].trim_end_matches(".git").to_string();
        let mut branch = String::from("main");
        let mut subpath = None;
        let remaining_path: Vec<&str> = segments[2..].to_vec();
        if remaining_path.len() >= 2 && remaining_path[0] == "tree" {
            branch = remaining_path[1].to_string();
            if remaining_path.len() > 2 {
                subpath = Some(remaining_path[2..].join("/"));
            }
        }
        Ok((owner, repository, branch, subpath))
    }
    async fn handle_github_response<T: for<'de> serde::Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, TextSourceError> {
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(TextSourceError::NetworkError)?;
        if status.is_success() {
            Ok(serde_json::from_str::<T>(&text)
                .map_err(|e| TextSourceError::GitHubError(e.to_string()))?)
        } else if status.as_u16() == 403 {
            Err(TextSourceError::RateLimitExceeded)
        } else if status.as_u16() == 404 {
            Err(TextSourceError::RepoNotFound)
        } else {
            Err(TextSourceError::GitHubError(text))
        }
    }
}

#[derive(serde::Deserialize)]
struct GitHubTreeResponse {
    tree: Vec<GitHubContent>,
}
#[derive(serde::Deserialize)]
struct GitHubContent {
    path: String,
    r#type: String,
}

#[async_trait]
impl TextSource for GitHubSource {
    async fn get_file_index(
        &self,
        filter: &FilterConfig,
    ) -> Result<Vec<SourceFile>, TextSourceError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
            self.owner, self.repo, self.branch
        );
        let response = self.client.get(&url).send().await?;
        let tree_response: GitHubTreeResponse = self.handle_github_response(response).await?;
        let filtered_contents = tree_response
            .tree
            .into_iter()
            .filter(|item| item.r#type == "blob")
            .filter(|item| {
                if let Some(sp) = &self.subpath {
                    if !item.path.starts_with(sp) {
                        return false;
                    }
                }
                true
            })
            .filter(|item| {
                let path_relative = if let Some(sp) = &self.subpath {
                    if let Some(stripped) = item.path.strip_prefix(sp) {
                        stripped.trim_start_matches('/')
                    } else {
                        &item.path
                    }
                } else {
                    &item.path
                };
                if let Some(ext) = crate::input::file_system::get_extension(path_relative) {
                    filter.is_text_extension(ext)
                } else {
                    true
                }
            })
            .map(|item| {
                let path_str = if let Some(sp) = &self.subpath {
                    if let Some(stripped) = item.path.strip_prefix(sp) {
                        stripped.trim_start_matches('/').to_string()
                    } else {
                        item.path
                    }
                } else {
                    item.path
                };
                SourceFile {
                    path: path_str,
                    source_type: super::SourceType::GitHub {
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
            super::SourceType::GitHub {
                owner,
                repo,
                branch,
            } => {
                let file_path = if let Some(sp) = self.subpath.as_ref() {
                    format!("{}/{}", sp, source_file.path)
                } else {
                    source_file.path.clone()
                };
                let raw_url = format!(
                    "https://raw.githubusercontent.com/{}/{}/{}/{}",
                    owner, repo, branch, file_path
                );
                let response = self.client.get(&raw_url).send().await?;
                let status = response.status();
                let bytes = response.bytes().await?;
                if status.is_success() {
                    Ok(String::from_utf8(bytes.to_vec())
                        .map_err(|_| TextSourceError::NotTextFile(file_path))?)
                } else if status.as_u16() == 404 {
                    Err(TextSourceError::PathNotFound(source_file.path.clone()))
                } else if status.as_u16() == 403 {
                    Err(TextSourceError::RateLimitExceeded)
                } else {
                    Err(TextSourceError::GitHubError(status.to_string()))
                }
            }
            _ => Err(TextSourceError::InvalidSource),
        }
    }
}
