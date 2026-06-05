use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Result, TelePiError};

/// A single entry from a session JSONL file.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: Option<String>,
    pub message: Option<EntryMessage>,
    #[serde(rename = "modelId")]
    pub model_id: Option<String>,
    pub provider: Option<String>,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: Option<String>,
    /// User-assigned label (stored in memory, not in JSONL).
    #[serde(skip)]
    pub label: Option<String>,
}

/// Message content within a session entry.
#[derive(Debug, Clone, Deserialize)]
pub struct EntryMessage {
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
}

/// A node in the session tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entry: SessionEntry,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Get a short description of this entry for display.
    pub fn describe(&self) -> String {
        match self.entry.entry_type.as_str() {
            "session" => "📋 Session start".to_string(),
            "model_change" => {
                let model = self.entry.model_id.as_deref().unwrap_or("unknown");
                let provider = self.entry.provider.as_deref().unwrap_or("");
                if provider.is_empty() {
                    format!("🤖 Model: {model}")
                } else {
                    format!("🤖 Model: {provider}/{model}")
                }
            }
            "thinking_level_change" => {
                let level = self.entry.thinking_level.as_deref().unwrap_or("unknown");
                format!("🧠 Thinking: {level}")
            }
            "message" => {
                if let Some(ref msg) = self.entry.message {
                    let role = msg.role.as_deref().unwrap_or("unknown");
                    let preview = extract_text_preview(msg);
                    match role {
                        "user" => format!("👤 {preview}"),
                        "assistant" => format!("🤖 {preview}"),
                        "toolResult" => format!("🔧 {preview}"),
                        _ => format!("💬 {role}: {preview}"),
                    }
                } else {
                    "💬 Message".to_string()
                }
            }
            other => format!("❓ {other}"),
        }
    }

    /// Get the label or describe text.
    pub fn display_text(&self) -> String {
        if let Some(ref label) = self.entry.label {
            format!("🏷️ {label}")
        } else {
            self.describe()
        }
    }
}

/// Extract a text preview from a message entry.
fn extract_text_preview(msg: &EntryMessage) -> String {
    let content = match &msg.content {
        Some(c) => c,
        None => return "(empty)".to_string(),
    };

    // content is usually an array of content blocks
    if let Some(arr) = content.as_array() {
        for block in arr {
            if let Some(obj) = block.as_object() {
                if let Some(t) = obj.get("type").and_then(|v| v.as_str()) {
                    match t {
                        "text" => {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                return truncate(text, 60);
                            }
                        }
                        "thinking" => {
                            if let Some(thinking) = obj.get("thinking").and_then(|v| v.as_str()) {
                                return format!("💭 {}", truncate(thinking, 50));
                            }
                        }
                        "tool_use" => {
                            if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                                return format!("🔧 {name}");
                            }
                        }
                        "toolResult" => {
                            if let Some(name) = obj.get("toolName").and_then(|v| v.as_str()) {
                                return format!("📋 {name} result");
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Fallback: try as string
    if let Some(s) = content.as_str() {
        return truncate(s, 60);
    }

    "(unknown)".to_string()
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

/// Parse a session JSONL file into a list of entries.
pub fn parse_session_jsonl(path: &Path) -> Result<Vec<SessionEntry>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| TelePiError::PiSession(format!("failed to read session file: {e}")))?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEntry>(trimmed) {
            Ok(entry) => entries.push(entry),
            Err(_) => continue, // Skip unparseable lines
        }
    }

    Ok(entries)
}

/// Build a tree from a flat list of entries.
pub fn build_tree(entries: Vec<SessionEntry>) -> Vec<TreeNode> {
    let mut node_map: HashMap<String, TreeNode> = HashMap::new();
    let mut root_ids: Vec<String> = Vec::new();

    // Create nodes
    for entry in entries {
        let id = entry.id.clone();
        let parent_id = entry.parent_id.clone();

        let node = TreeNode {
            entry,
            children: Vec::new(),
        };

        if parent_id.is_none() {
            root_ids.push(id.clone());
        }

        node_map.insert(id, node);
    }

    // Build parent-child relationships
    let child_ids: Vec<(String, String)> = node_map
        .iter()
        .filter_map(|(id, node)| {
            node.entry
                .parent_id
                .as_ref()
                .map(|pid| (pid.clone(), id.clone()))
        })
        .collect();

    for (parent_id, child_id) in child_ids {
        if let Some(child) = node_map.remove(&child_id) {
            if let Some(parent) = node_map.get_mut(&parent_id) {
                parent.children.push(child);
            }
        }
    }

    // Collect roots
    root_ids
        .into_iter()
        .filter_map(|id| node_map.remove(&id))
        .collect()
}

/// Render a tree as text with box-drawing characters.
pub fn render_tree(nodes: &[TreeNode], max_depth: usize, max_entries: usize) -> String {
    let mut output = String::new();
    let mut count = 0;
    render_tree_inner(nodes, &mut output, "", max_depth, &mut count, max_entries);
    output
}

fn render_tree_inner(
    nodes: &[TreeNode],
    output: &mut String,
    prefix: &str,
    max_depth: usize,
    count: &mut usize,
    max_entries: usize,
) {
    if *count >= max_entries || max_depth == 0 {
        return;
    }

    for (i, node) in nodes.iter().enumerate() {
        if *count >= max_entries {
            output.push_str(&format!("{prefix}└── … (more entries)\n"));
            return;
        }

        let is_last = i == nodes.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let label = node.display_text();
        let entry_id = &node.entry.id[..8.min(node.entry.id.len())];
        output.push_str(&format!("{prefix}{connector}{label} [{entry_id}]\n"));
        *count += 1;

        if !node.children.is_empty() {
            render_tree_inner(
                &node.children,
                output,
                &format!("{prefix}{child_prefix}"),
                max_depth - 1,
                count,
                max_entries,
            );
        }
    }
}

/// Find all session directories for a workspace.
pub fn find_session_dirs(workspace: &Path) -> Vec<PathBuf> {
    let encoded = encode_workspace_path(workspace);
    let sessions_base = dirs::home_dir()
        .unwrap_or_default()
        .join(".pi")
        .join("agent")
        .join("sessions")
        .join(&encoded);

    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&sessions_base) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                dirs.push(entry.path());
            }
        }
    }

    dirs.sort();
    dirs
}

/// Find the latest session.jsonl in a session directory.
pub fn find_latest_session_file(session_dir: &Path) -> Option<PathBuf> {
    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;

    // Walk: session_dir/<uuid>/run-N/session.jsonl
    if let Ok(entries) = std::fs::read_dir(session_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Look for run-N directories
            if let Ok(runs) = std::fs::read_dir(&path) {
                for run in runs.flatten() {
                    let run_path = run.path();
                    if !run_path.is_dir() {
                        continue;
                    }
                    let jsonl = run_path.join("session.jsonl");
                    if jsonl.exists() {
                        if let Ok(meta) = std::fs::metadata(&jsonl) {
                            let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                            if latest.is_none() || modified > latest.as_ref().unwrap().0 {
                                latest = Some((modified, jsonl));
                            }
                        }
                    }
                }
            }
        }
    }

    latest.map(|(_, path)| path)
}

/// Encode a workspace path to the directory name format used by Pi.
fn encode_workspace_path(workspace: &Path) -> String {
    let s = workspace.to_string_lossy();
    let mut encoded = s.replace('/', "-");
    if !encoded.starts_with('-') {
        encoded = format!("-{encoded}");
    }
    if !encoded.ends_with('-') {
        encoded.push('-');
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello…");
    }

    #[test]
    fn test_encode_workspace_path() {
        let path = PathBuf::from("/Users/test/code");
        let encoded = encode_workspace_path(&path);
        assert!(encoded.starts_with('-'));
        assert!(encoded.ends_with('-'));
    }
}
