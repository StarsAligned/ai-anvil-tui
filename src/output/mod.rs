pub mod clipboard;
pub mod file;
use crate::input::SourceFile;
use crate::ui::output::OutputDestination;
use crate::ui::App;
use std::collections::HashMap;

pub async fn write_merged(
    destination: &OutputDestination,
    output_file: &str,
    files_map: HashMap<String, SourceFile>,
    app: &mut App,
) -> Result<String, String> {
    let mut merged = String::new();
    for (path, sf) in files_map {
        if let Ok(content) = app.reload_file_content(&sf).await {
            merged.push_str(&format!("--- START FILE: {} ---\n", path));
            merged.push_str(&content);
            merged.push_str(&format!("\n--- END FILE: {} ---\n\n", path));
        }
    }
    match destination {
        OutputDestination::FileAndClipboard | OutputDestination::File => {
            if let Err(e) = file::write_file(output_file, &merged) {
                return Err(e);
            }
        }
        _ => {}
    }
    if matches!(
        destination,
        OutputDestination::FileAndClipboard | OutputDestination::Clipboard
    ) {
        if let Err(e) = clipboard::copy_clipboard(merged.clone()) {
            return Err(e);
        }
    }
    Ok(merged)
}
