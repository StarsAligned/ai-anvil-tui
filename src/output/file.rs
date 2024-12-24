use std::fs;
use std::io::Write;

pub fn write_file(path: &str, content: &str) -> Result<(), String> {
    let mut file = fs::File::create(path).map_err(|e| format!("Error creating file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Error writing file: {}", e))?;
    Ok(())
}
