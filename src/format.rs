use regex::Regex;
use std::sync::LazyLock;

/// Escape HTML special characters for Telegram messages.
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Convert Markdown to Telegram HTML.
///
/// Supports: code blocks, inline code, bold, italic, links, blockquotes.
pub fn markdown_to_telegram_html(markdown: &str) -> String {
    // Step 1: Escape HTML entities in the raw markdown
    let mut text = escape_html(markdown);

    // Step 2: Extract and protect code blocks (```...```)
    let mut code_blocks: Vec<String> = Vec::new();
    static CODE_BLOCK_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)```(\w*)\n?(.*?)```").unwrap());

    text = CODE_BLOCK_RE
        .replace_all(&text, |caps: &regex::Captures| {
            let lang = &caps[1];
            let code = &caps[2];
            let idx = code_blocks.len();
            let placeholder = format!("\u{E000}CB{idx}\u{E000}");
            if lang.is_empty() {
                code_blocks.push(format!("<pre>{}</pre>", code.trim()));
            } else {
                code_blocks.push(format!(
                    "<pre><code class=\"language-{lang}\">{}</code></pre>",
                    code.trim()
                ));
            }
            placeholder
        })
        .into_owned();

    // Step 3: Extract and protect inline code (`...`)
    let mut inline_codes: Vec<String> = Vec::new();
    static INLINE_CODE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`([^`\n]+)`").unwrap());

    text = INLINE_CODE_RE
        .replace_all(&text, |caps: &regex::Captures| {
            let idx = inline_codes.len();
            let placeholder = format!("\u{E000}IC{idx}\u{E000}");
            inline_codes.push(format!("<code>{}</code>", &caps[1]));
            placeholder
        })
        .into_owned();

    // Step 4: Bold (**text** or __text__)
    static BOLD_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\*\*(.+?)\*\*|__(.+?)__").unwrap());
    text = BOLD_RE
        .replace_all(&text, |caps: &regex::Captures| {
            let content = caps.get(1).or(caps.get(2)).unwrap().as_str();
            format!("<b>{content}</b>")
        })
        .into_owned();

    // Step 5: Italic (*text* or _text_)
    static ITALIC_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\*([^*\n]+?)\*").unwrap());
    text = ITALIC_RE
        .replace_all(&text, |caps: &regex::Captures| {
            let content = caps.get(1).or(caps.get(2)).unwrap().as_str();
            format!("<i>{content}</i>")
        })
        .into_owned();

    // Step 6: Links [text](url)
    static LINK_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());
    text = LINK_RE
        .replace_all(&text, |caps: &regex::Captures| {
            let link_text = &caps[1];
            let url = &caps[2];
            format!("<a href=\"{url}\">{link_text}</a>")
        })
        .into_owned();

    // Step 7: Blockquotes (> text)
    static BLOCKQUOTE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^&gt;\s?(.*)$").unwrap());
    text = BLOCKQUOTE_RE
        .replace_all(&text, |caps: &regex::Captures| {
            format!("<blockquote>{}</blockquote>", &caps[1])
        })
        .into_owned();

    // Step 8: Restore inline code placeholders
    for (idx, code) in inline_codes.iter().enumerate() {
        text = text.replace(&format!("\u{E000}IC{idx}\u{E000}"), code);
    }

    // Step 9: Restore code block placeholders
    for (idx, block) in code_blocks.iter().enumerate() {
        text = text.replace(&format!("\u{E000}CB{idx}\u{E000}"), block);
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("<b>bold</b>"), "&lt;b&gt;bold&lt;/b&gt;");
    }

    #[test]
    fn test_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let result = markdown_to_telegram_html(input);
        assert!(result.contains("<pre>"));
        assert!(result.contains("fn main() {}"));
    }

    #[test]
    fn test_inline_code() {
        let input = "use `cargo build` to compile";
        let result = markdown_to_telegram_html(input);
        assert!(result.contains("<code>cargo build</code>"));
    }

    #[test]
    fn test_bold() {
        let input = "this is **bold** text";
        let result = markdown_to_telegram_html(input);
        assert!(result.contains("<b>bold</b>"));
    }

    #[test]
    fn test_link() {
        let input = "[Rust](https://rust-lang.org)";
        let result = markdown_to_telegram_html(input);
        assert!(result.contains("<a href=\"https://rust-lang.org\">Rust</a>"));
    }
}
