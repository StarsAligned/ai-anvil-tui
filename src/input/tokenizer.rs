use log::info;
use tiktoken_rs::o200k_base;

pub fn count_tokens_in_content(content: &str) -> Result<usize, String> {
	info!("Starting token count for content of length {}", content.len());
	let bpe = o200k_base().map_err(|e| e.to_string())?;
	let tokens = bpe.encode_with_special_tokens(content);
	info!("Token counting complete, total tokens: {}", tokens.len());
	Ok(tokens.len())
}