use async_trait::async_trait;
use std::path::Path;

use crate::domain::error::DomainError;
use crate::domain::ir::{DocumentBuilder, ParsedBlock, ParsedSource};
use crate::domain::parser::FileParserBackend;

/// DOCX parser that extracts text from Word documents
pub struct DocxParser;

impl DocxParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocxParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileParserBackend for DocxParser {
    fn id(&self) -> &'static str {
        "docx"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["docx"]
    }

    async fn parse_local_path(
        &self,
        path: &Path,
    ) -> Result<crate::domain::ir::ParsedDocument, DomainError> {
        let path_buf = path.to_path_buf();

        let blocks =
            tokio::task::spawn_blocking(move || -> Result<Vec<ParsedBlock>, DomainError> {
                let bytes = std::fs::read(&path_buf)
                    .map_err(|e| DomainError::io_error(format!("Failed to read file: {}", e)))?;

                let docx = docx_rs::read_docx(&bytes).map_err(|e| {
                    DomainError::parse_error(format!("Failed to parse DOCX: {}", e))
                })?;

                Ok(extract_blocks_from_docx(&docx))
            })
            .await
            .map_err(|e| DomainError::parse_error(format!("Task join error: {}", e)))??;

        let mut builder = DocumentBuilder::new(ParsedSource::LocalPath(path.display().to_string()))
            .content_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
            .blocks(blocks);

        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
            builder = builder.title(filename).original_filename(filename);
        }

        Ok(builder.build())
    }

    async fn parse_bytes(
        &self,
        filename_hint: Option<&str>,
        _content_type: Option<&str>,
        bytes: bytes::Bytes,
    ) -> Result<crate::domain::ir::ParsedDocument, DomainError> {
        let blocks =
            tokio::task::spawn_blocking(move || -> Result<Vec<ParsedBlock>, DomainError> {
                let docx = docx_rs::read_docx(&bytes).map_err(|e| {
                    DomainError::parse_error(format!("Failed to parse DOCX: {}", e))
                })?;

                Ok(extract_blocks_from_docx(&docx))
            })
            .await
            .map_err(|e| DomainError::parse_error(format!("Task join error: {}", e)))??;

        let source = ParsedSource::Uploaded {
            original_name: filename_hint.unwrap_or("unknown.docx").to_string(),
        };

        let mut builder = DocumentBuilder::new(source)
            .content_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
            .blocks(blocks);

        if let Some(filename) = filename_hint {
            builder = builder.title(filename).original_filename(filename);
        }

        Ok(builder.build())
    }
}

fn extract_blocks_from_docx(docx: &docx_rs::Docx) -> Vec<ParsedBlock> {
    let mut blocks = Vec::new();

    for child in &docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(para) = child {
            let mut text = String::new();

            for para_child in &para.children {
                if let docx_rs::ParagraphChild::Run(run) = para_child {
                    for run_child in &run.children {
                        if let docx_rs::RunChild::Text(text_elem) = run_child {
                            text.push_str(&text_elem.text);
                        }
                    }
                }
            }

            if !text.trim().is_empty() {
                // TODO: Detect headings based on paragraph style
                blocks.push(ParsedBlock::Paragraph { text });
            }
        }
    }

    blocks
}
