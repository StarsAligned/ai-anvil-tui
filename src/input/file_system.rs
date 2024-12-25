use super::{FilterConfig, SourceFile, SourceType, TextSource, TextSourceError};
use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_extension(path_str: &str) -> Option<String> {
    let p = Path::new(path_str);
    p.extension()
        .and_then(|ext| ext.to_str())
        .map(|e| e.to_lowercase())
}

struct GitIgnoreRules {
    patterns: Vec<String>,
}

impl GitIgnoreRules {
    fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }
    fn load_from(base_path: &Path) -> Self {
        let mut rules = Self::new();
        let ignore_path = base_path.join(".gitignore");
        if ignore_path.exists() && ignore_path.is_file() {
            if let Ok(content) = fs::read_to_string(&ignore_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    rules.patterns.push(trimmed.into());
                }
            }
        }
        rules
    }
    fn is_ignored(&self, rel_path: &str) -> bool {
        for p in &self.patterns {
            if self.match_pattern(rel_path, p) {
                return true;
            }
        }
        false
    }
    fn match_pattern(&self, rel_path: &str, pat: &str) -> bool {
        let trimmed_pat = pat.trim_end_matches('/');
        if pat.starts_with('/') {
            let pat_no_slash = trimmed_pat.trim_start_matches('/');
            if rel_path == pat_no_slash {
                return true;
            }
            if rel_path.starts_with(&format!("{}/", pat_no_slash)) {
                return true;
            }
            if pat_no_slash.starts_with('*') {
                let after_star = pat_no_slash.trim_start_matches('*');
                if rel_path.ends_with(after_star) {
                    return true;
                }
            }
        } else if pat.contains('*') {
            let star_idx = pat.find('*').unwrap();
            let (start, end) = pat.split_at(star_idx);
            let after_star = &end[1..];
            if rel_path.starts_with(start) && rel_path.ends_with(after_star) {
                return true;
            }
        } else {
            if rel_path == trimmed_pat {
                return true;
            }
            if rel_path.starts_with(&format!("{}/", trimmed_pat)) {
                return true;
            }
        }
        false
    }
}

pub struct FileSystemSource {
    pub base_path: PathBuf,
    gitignore_rules: GitIgnoreRules,
}

impl FileSystemSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, TextSourceError> {
        let base_path = path.as_ref().to_path_buf();
        if !base_path.exists() {
            return Err(TextSourceError::PathNotFound(
                base_path.to_string_lossy().to_string(),
            ));
        }
        if !base_path.is_dir() {
            return Err(TextSourceError::InvalidSource);
        }
        fs::read_dir(&base_path).map_err(|_| {
            TextSourceError::PermissionDenied(base_path.to_string_lossy().to_string())
        })?;
        let gitignore_rules = GitIgnoreRules::load_from(&base_path);
        Ok(Self {
            base_path,
            gitignore_rules,
        })
    }
    fn collect_files(
        &self,
        dir: &Path,
        files: &mut Vec<SourceFile>,
        filter: &FilterConfig,
    ) -> Result<(), TextSourceError> {
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path
                .strip_prefix(&self.base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            if let Some(fname) = path.file_name() {
                let fname_str = fname.to_string_lossy();
                if fname_str.starts_with('.') {
                    continue;
                }
                if fname_str.ends_with('~') {
                    continue;
                }
            }
            if self.gitignore_rules.is_ignored(&rel_path) {
                continue;
            }
            if let Some(ext) = get_extension(&rel_path) {
                if !filter.is_text_extension(ext) {
                    continue;
                }
            }
            if path.is_file() {
                files.push(SourceFile {
                    path: rel_path,
                    source_type: SourceType::FileSystem {
                        base_path: self.base_path.clone(),
                    },
                });
            } else if path.is_dir() {
                self.collect_files(&path, files, filter)?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TextSource for FileSystemSource {
    async fn get_file_index(
        &self,
        filter: &FilterConfig,
    ) -> Result<Vec<SourceFile>, TextSourceError> {
        let mut files = Vec::new();
        self.collect_files(&self.base_path, &mut files, filter)?;
        Ok(files)
    }
    async fn get_file_content(&self, source_file: &SourceFile) -> Result<String, TextSourceError> {
        if let SourceType::FileSystem { base_path } = &source_file.source_type {
            let full_path = base_path.join(&source_file.path);
            if !full_path.exists() {
                return Err(TextSourceError::PathNotFound(
                    full_path.to_string_lossy().to_string(),
                ));
            }
            let bytes = fs::read(&full_path)?;
            Ok(String::from_utf8(bytes).map_err(|_| {
                TextSourceError::NotTextFile(full_path.to_string_lossy().to_string())
            })?)
        } else {
            Err(TextSourceError::InvalidSource)
        }
    }
}
