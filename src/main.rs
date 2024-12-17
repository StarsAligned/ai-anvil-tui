mod text_source;

use crate::text_source::{create_text_source, FilterConfig, TextSourceError};

#[tokio::main]
async fn main() -> Result<(), TextSourceError> {
    let filter = FilterConfig::new()
        .with_show_hidden(false);

    let source = create_text_source("./").await?;
    let files = source.get_file_index(&filter).await?;

    for file in files {
        //println!("File: {}", file.path);
        match source.get_file_content(&file).await {
            Ok(content) => {  },//println!("Content length: {} bytes", content.len()),
            Err(TextSourceError::NotTextFile(path)) => {
                println!("Skipping non-text file: {}", path);
                continue;
            }
            Err(e) => {
                println!("Error reading file {}: {}", file.path, e);
                continue;
            }
        }
    }

    Ok(())
}