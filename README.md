# AI Anvil TUI

AI Anvil TUI is a terminal-based tool that helps you quickly gather and merge text-based files, like project sources or documentation, so you can provide your AI chat sessions with the most current information.
By merging each relevant file into a single text output, you can easily paste or upload the entire context to the AI, ensuring it references the right version of your code or libraries.

--------------------------------------------------------------------------------

## Overview

• Merge local project code to share with an AI chat for debugging, refactoring, or feature suggestions.  
• Include up-to-date documentation from frameworks or libraries so the AI does not confuse newer features with older versions.  
• Count tokens (o200k_base) to keep file size within AI model limits before copying or saving.  
• Dynamically load files from a local directory or straight from GitHub repositories.

--------------------------------------------------------------------------------

## Key Features

1. **Load Local or GitHub Sources**  
   Specify a path on your system, or pull files from a GitHub repository by URL (e.g., https://github.com/owner/repo/tree/branch/subpath).  
   The tool detects text-based files and filters out binaries.

2. **Filter by Extension**  
   Choose which file extensions to include, or toggle entire groups of files (like “.md” or “.toml”).  

3. **Token Counting**  
   Get an estimate of token usage for your selected files, leveraging [tiktoken-rs](https://github.com/itdxer/tiktoken-rs).  
   Useful for validating your input size against model limits.

4. **Merge & Clipboard Support**  
   Merge selected files into a single text.  
   Optionally write to disk and/or copy to your clipboard, simplifying the process of sending text to an AI chat.

--------------------------------------------------------------------------------

## Installing & Running

1. **Prerequisites**  
   • (Optional) Rust 1.60+ if you want to compile from source.  
   • A clipboard manager is often supported by default on most systems.

2. **Download or Build**  
   - To build locally:  
     git clone https://github.com/your-user/ai-anvil-tui.git  
     cd ai-anvil-tui  
     cargo build --release  

     Your built binary will be in target/release (e.g., ai-anvil-tui (Linux/Mac) or ai-anvil-tui.exe (Windows)).

   - For a pre-built release, you might see files like ai-anvil-tui-0.2.1-win64.zip. Unzip and run the .exe inside.

3. **Usage Examples**  

   • Run with no parameters (opens in your current directory):  
     ai-anvil-tui-0.2.1-win64.exe  

   • Run with a local path:  
     ai-anvil-tui-0.2.1-win64.exe /path/to/my/project  

   • Run with a GitHub repo:  
     ai-anvil-tui-0.2.1-win64.exe https://github.com/owner/repo/tree/main/some-subdirectory  

--------------------------------------------------------------------------------

## Interface Guide

1. **Source Panel**  
   - Type a local path or GitHub URL, then press Enter to proceed.  

2. **Filters Panel**  
   - Press Space to toggle inclusion of file extensions. Press Enter to move on.  

3. **Source Files Panel**  
   - Shows all files based on your filter.  
   - Toggle individual files with Space, press Enter to confirm and see token counts.

4. **Output Panel**  
   - Choose if you want just a file, just the clipboard, or both.  
   - Press Enter, or press F2 for immediate merging if you picked clipboard-only.

5. **Output File Panel**  
   - If merging to a file, specify its path/name (e.g. “./merged_context.txt”). Press Enter or F2 to finalize.

--------------------------------------------------------------------------------

## Shortcuts & Controls

• F1 = Reload file list  
• F2 = Merge selected files  
• F3 = Clear current text input (source path or output filename)  
• Esc = Go back one panel or exit if on the first panel  
• F10 = Quit the TUI from any panel  

Navigation keys:  
• Arrow keys (Up/Down) for scrolling/filter changes/selection  
• Left/Right in the Output panel to toggle destinations  
• Space in Filters or Source Files to select/deselect

--------------------------------------------------------------------------------

## Contributing & Internals

• TUI built with [Ratatui](https://docs.rs/ratatui) and [crossterm](https://docs.rs/crossterm).  
• Token counting powered by [tiktoken-rs](https://github.com/itdxer/tiktoken-rs).  
• The code is structured into modules under src/, including “ui”, “input”, and “output”.  
• Contributions, feature requests, and bug reports are always welcome.

--------------------------------------------------------------------------------

## Limitations / Notes

• Very large GitHub repos may hit rate limits.  
• Non-UTF8 files are skipped.  
• .gitignore logic is approximate and may not match Git’s own behavior exactly.

--------------------------------------------------------------------------------

Use AI Anvil TUI to streamline copying just the right files for your AI-based development chats or ensuring your AI assistant sees the latest library documentation!
