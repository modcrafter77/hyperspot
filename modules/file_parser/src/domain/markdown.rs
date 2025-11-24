use crate::domain::ir::{ParsedBlock, ParsedDocument};

/// Markdown renderer that converts ParsedDocument to Markdown string
pub struct MarkdownRenderer;

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    /// Create a new markdown renderer
    pub fn new() -> Self {
        Self
    }

    /// Render a document using this renderer instance
    pub fn render_doc(&self, doc: &ParsedDocument) -> String {
        Self::render(doc)
    }

    /// Render a parsed document to Markdown (static method)
    pub fn render(doc: &ParsedDocument) -> String {
        let mut output = String::new();

        // Render title if present
        if let Some(ref title) = doc.title {
            output.push_str("# ");
            output.push_str(title);
            output.push_str("\n\n");
        }

        // Render metadata section if we have useful info
        if doc.language.is_some()
            || doc.meta.original_filename.is_some()
            || doc.meta.content_type.is_some()
        {
            output.push_str("---\n");
            if let Some(ref lang) = doc.language {
                output.push_str(&format!("language: {}\n", lang));
            }
            if let Some(ref filename) = doc.meta.original_filename {
                output.push_str(&format!("filename: {}\n", filename));
            }
            if let Some(ref content_type) = doc.meta.content_type {
                output.push_str(&format!("content-type: {}\n", content_type));
            }
            output.push_str("---\n\n");
        }

        // Render blocks
        for block in &doc.blocks {
            Self::render_block(block, &mut output);
        }

        output
    }

    fn render_block(block: &ParsedBlock, output: &mut String) {
        match block {
            ParsedBlock::Heading { level, text } => {
                let level = (*level).clamp(1, 6);
                output.push_str(&"#".repeat(level as usize));
                output.push(' ');
                output.push_str(text);
                output.push_str("\n\n");
            }
            ParsedBlock::Paragraph { text } => {
                output.push_str(text);
                output.push_str("\n\n");
            }
            ParsedBlock::ListItem {
                level,
                ordered,
                text,
            } => {
                // Add indentation
                let indent = "  ".repeat(*level as usize);
                output.push_str(&indent);

                // Add bullet or number
                if *ordered {
                    output.push_str("1. ");
                } else {
                    output.push_str("- ");
                }

                output.push_str(text);
                output.push('\n');
            }
            ParsedBlock::CodeBlock { language, code } => {
                output.push_str("```");
                if let Some(lang) = language {
                    output.push_str(lang);
                }
                output.push('\n');
                output.push_str(code);
                if !code.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("```\n\n");
            }
            ParsedBlock::Table { markdown } => {
                output.push_str(markdown);
                output.push_str("\n\n");
            }
            ParsedBlock::Quote { text } => {
                for line in text.lines() {
                    output.push_str("> ");
                    output.push_str(line);
                    output.push('\n');
                }
                output.push('\n');
            }
            ParsedBlock::HorizontalRule => {
                output.push_str("---\n\n");
            }
            ParsedBlock::Image { alt, title, src } => {
                output.push('!');
                output.push('[');
                if let Some(alt_text) = alt {
                    output.push_str(alt_text);
                }
                output.push(']');
                output.push('(');
                if let Some(source) = src {
                    output.push_str(source);
                }
                if let Some(title_text) = title {
                    output.push_str(" \"");
                    output.push_str(title_text);
                    output.push('"');
                }
                output.push(')');
                output.push_str("\n\n");
            }
            ParsedBlock::PageBreak => {
                output.push_str("\n\n---\n\n");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ir::{ParsedMetadata, ParsedSource};

    #[test]
    fn test_render_heading() {
        let doc = ParsedDocument {
            id: None,
            title: None,
            language: None,
            meta: ParsedMetadata {
                source: ParsedSource::LocalPath("test.txt".to_string()),
                original_filename: None,
                content_type: None,
                created_at: None,
                modified_at: None,
                is_stub: false,
            },
            blocks: vec![
                ParsedBlock::Heading {
                    level: 1,
                    text: "Title".to_string(),
                },
                ParsedBlock::Heading {
                    level: 2,
                    text: "Subtitle".to_string(),
                },
            ],
        };

        let markdown = MarkdownRenderer::render(&doc);
        assert!(markdown.contains("# Title\n"));
        assert!(markdown.contains("## Subtitle\n"));
    }

    #[test]
    fn test_render_paragraph() {
        let doc = ParsedDocument {
            id: None,
            title: None,
            language: None,
            meta: ParsedMetadata {
                source: ParsedSource::LocalPath("test.txt".to_string()),
                original_filename: None,
                content_type: None,
                created_at: None,
                modified_at: None,
                is_stub: false,
            },
            blocks: vec![ParsedBlock::Paragraph {
                text: "Hello world".to_string(),
            }],
        };

        let markdown = MarkdownRenderer::render(&doc);
        assert!(markdown.contains("Hello world\n"));
    }

    #[test]
    fn test_render_list() {
        let doc = ParsedDocument {
            id: None,
            title: None,
            language: None,
            meta: ParsedMetadata {
                source: ParsedSource::LocalPath("test.txt".to_string()),
                original_filename: None,
                content_type: None,
                created_at: None,
                modified_at: None,
                is_stub: false,
            },
            blocks: vec![
                ParsedBlock::ListItem {
                    level: 0,
                    ordered: false,
                    text: "Item 1".to_string(),
                },
                ParsedBlock::ListItem {
                    level: 1,
                    ordered: false,
                    text: "Nested item".to_string(),
                },
            ],
        };

        let markdown = MarkdownRenderer::render(&doc);
        assert!(markdown.contains("- Item 1\n"));
        assert!(markdown.contains("  - Nested item\n"));
    }

    #[test]
    fn test_render_code_block() {
        let doc = ParsedDocument {
            id: None,
            title: None,
            language: None,
            meta: ParsedMetadata {
                source: ParsedSource::LocalPath("test.txt".to_string()),
                original_filename: None,
                content_type: None,
                created_at: None,
                modified_at: None,
                is_stub: false,
            },
            blocks: vec![ParsedBlock::CodeBlock {
                language: Some("rust".to_string()),
                code: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            }],
        };

        let markdown = MarkdownRenderer::render(&doc);
        assert!(markdown.contains("```rust\n"));
        assert!(markdown.contains("fn main()"));
    }

    #[test]
    fn test_render_with_title() {
        let doc = ParsedDocument {
            id: None,
            title: Some("Document Title".to_string()),
            language: None,
            meta: ParsedMetadata {
                source: ParsedSource::LocalPath("test.txt".to_string()),
                original_filename: None,
                content_type: None,
                created_at: None,
                modified_at: None,
                is_stub: false,
            },
            blocks: vec![ParsedBlock::Paragraph {
                text: "Content".to_string(),
            }],
        };

        let markdown = MarkdownRenderer::render(&doc);
        assert!(markdown.starts_with("# Document Title\n"));
    }
}
