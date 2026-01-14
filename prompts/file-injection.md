# File Injection Markers

When file contents are injected into prompts via workflow variables (type: "file"), they are wrapped with special delimiters to clearly mark the boundaries:

```
///!agent:injected_file_start:<path>
<file contents>
///!agent:injected_file_end:<path>
```

## Example

If a workflow variable injects `CLAUDE.md`:

```
///!agent:injected_file_start:CLAUDE.md
# CLAUDE.md

Keep this file updated when making architectural changes...
///!agent:injected_file_end:CLAUDE.md
```

## Purpose

These markers help you:
- Identify which content came from external files
- Distinguish between multiple injected files
- Reference specific files in your responses
- Understand the source of context provided to you

## Usage

When responding to prompts that contain injected files:
- You can reference the file by its path (shown in the markers)
- The content between markers is the complete file contents
- Multiple files may be injected in the same prompt
