use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ir;
use crate::domain::service::FileParserInfo;

/// REST DTO for file parser info response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileParserInfoDto {
    pub supported_extensions: HashMap<String, Vec<String>>,
}

/// REST DTO for parse local file request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseLocalRequest {
    pub file_path: String,
}

/// REST DTO for parse URL request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseUrlRequest {
    pub url: String,
}

/// REST DTO for parsed document metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParsedMetadataDto {
    pub source: ParsedSourceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_stub: bool,
}

/// REST DTO for document source
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParsedSourceDto {
    LocalPath { path: String },
    Uploaded { original_name: String },
    Url { url: String },
}

/// REST DTO for parsed block
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParsedBlockDto {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    ListItem {
        level: u8,
        ordered: bool,
        text: String,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Table {
        markdown: String,
    },
    Quote {
        text: String,
    },
    HorizontalRule,
    Image {
        alt: Option<String>,
        title: Option<String>,
        src: Option<String>,
    },
    PageBreak,
}

/// REST DTO for parsed document (IR)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParsedDocumentDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub meta: ParsedMetadataDto,
    pub blocks: Vec<ParsedBlockDto>,
}

/// REST DTO for file parse response (with optional markdown)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileParseResponseDto {
    /// The parsed document in intermediate representation
    pub document: ParsedDocumentDto,
    /// Rendered markdown (only present when render_markdown=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

// Conversion implementations

impl From<FileParserInfo> for FileParserInfoDto {
    fn from(info: FileParserInfo) -> Self {
        Self {
            supported_extensions: info.supported_extensions,
        }
    }
}

impl From<ir::ParsedDocument> for ParsedDocumentDto {
    fn from(doc: ir::ParsedDocument) -> Self {
        Self {
            id: doc.id,
            title: doc.title,
            language: doc.language,
            meta: doc.meta.into(),
            blocks: doc.blocks.into_iter().map(|b| b.into()).collect(),
        }
    }
}

impl From<ir::ParsedMetadata> for ParsedMetadataDto {
    fn from(meta: ir::ParsedMetadata) -> Self {
        Self {
            source: meta.source.into(),
            original_filename: meta.original_filename,
            content_type: meta.content_type,
            created_at: meta.created_at,
            modified_at: meta.modified_at,
            is_stub: meta.is_stub,
        }
    }
}

impl From<ir::ParsedSource> for ParsedSourceDto {
    fn from(source: ir::ParsedSource) -> Self {
        match source {
            ir::ParsedSource::LocalPath(path) => ParsedSourceDto::LocalPath { path },
            ir::ParsedSource::Uploaded { original_name } => {
                ParsedSourceDto::Uploaded { original_name }
            }
            ir::ParsedSource::Url(url) => ParsedSourceDto::Url { url },
        }
    }
}

impl From<ir::ParsedBlock> for ParsedBlockDto {
    fn from(block: ir::ParsedBlock) -> Self {
        match block {
            ir::ParsedBlock::Heading { level, text } => ParsedBlockDto::Heading { level, text },
            ir::ParsedBlock::Paragraph { text } => ParsedBlockDto::Paragraph { text },
            ir::ParsedBlock::ListItem {
                level,
                ordered,
                text,
            } => ParsedBlockDto::ListItem {
                level,
                ordered,
                text,
            },
            ir::ParsedBlock::CodeBlock { language, code } => {
                ParsedBlockDto::CodeBlock { language, code }
            }
            ir::ParsedBlock::Table { markdown } => ParsedBlockDto::Table { markdown },
            ir::ParsedBlock::Quote { text } => ParsedBlockDto::Quote { text },
            ir::ParsedBlock::HorizontalRule => ParsedBlockDto::HorizontalRule,
            ir::ParsedBlock::Image { alt, title, src } => ParsedBlockDto::Image { alt, title, src },
            ir::ParsedBlock::PageBreak => ParsedBlockDto::PageBreak,
        }
    }
}
