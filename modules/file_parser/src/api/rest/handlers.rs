use axum::extract::{Extension, Query};
use tracing::{field::Empty, info};

use crate::api::rest::dto::{
    FileParseResponseDto, FileParserInfoDto, ParseLocalRequest, ParseUrlRequest, ParsedDocumentDto,
};
use crate::domain::markdown::MarkdownRenderer;
use modkit::api::prelude::*;

use crate::domain::service::FileParserService;

// Import auth extractors
use modkit_auth::axum_ext::Authz;

// Type aliases for our specific API with DomainError
use crate::domain::error::DomainError;
type FileParserResult<T> = ApiResult<T, DomainError>;
type FileParserApiError = ApiError<DomainError>;

/// Query parameter for render_markdown flag
#[derive(Debug, serde::Deserialize)]
pub struct RenderMarkdownQuery {
    #[serde(default)]
    pub render_markdown: Option<bool>,
}

/// Get information about available file parsers
#[tracing::instrument(
    name = "file_parser.get_parser_info",
    skip(svc, _ctx),
    fields(
        request_id = Empty
    )
)]
#[axum::debug_handler]
pub async fn get_parser_info(
    Authz(_ctx): Authz,
    Extension(svc): Extension<std::sync::Arc<FileParserService>>,
) -> FileParserResult<JsonBody<FileParserInfoDto>> {
    info!("Getting file parser info");

    let info = svc.info();

    Ok(Json(FileParserInfoDto::from(info)))
}

/// Parse a file from a local path
#[tracing::instrument(
    name = "file_parser.parse_local",
    skip(svc, req_body, _ctx, query),
    fields(
        file_path = %req_body.file_path,
        render_markdown = ?query.render_markdown,
        request_id = Empty
    )
)]
#[axum::debug_handler]
pub async fn parse_local(
    Authz(_ctx): Authz,
    Extension(svc): Extension<std::sync::Arc<FileParserService>>,
    Query(query): Query<RenderMarkdownQuery>,
    Json(req_body): Json<ParseLocalRequest>,
) -> FileParserResult<JsonBody<FileParseResponseDto>> {
    let render_md = query.render_markdown.unwrap_or(false);

    info!(
        file_path = %req_body.file_path,
        render_markdown = render_md,
        "Parsing file from local path"
    );

    let path = std::path::Path::new(&req_body.file_path);
    let document = svc
        .parse_local(path)
        .await
        .map_err(FileParserApiError::from_domain)?;

    // Optionally render markdown
    let markdown = if render_md {
        Some(MarkdownRenderer::render(&document))
    } else {
        None
    };

    let response = FileParseResponseDto {
        document: ParsedDocumentDto::from(document),
        markdown,
    };

    Ok(Json(response))
}

/// Upload and parse a file
#[tracing::instrument(
    name = "file_parser.upload",
    skip(svc, multipart, _ctx, query),
    fields(
        render_markdown = ?query.render_markdown,
        request_id = Empty
    )
)]
#[axum::debug_handler]
pub async fn upload_and_parse(
    Authz(_ctx): Authz,
    Extension(svc): Extension<std::sync::Arc<FileParserService>>,
    Query(query): Query<RenderMarkdownQuery>,
    mut multipart: axum::extract::Multipart,
) -> FileParserResult<JsonBody<FileParseResponseDto>> {
    let render_md = query.render_markdown.unwrap_or(false);

    info!(render_markdown = render_md, "Uploading and parsing file");

    // Extract the first file field
    let mut file_name: Option<String> = None;
    let mut file_bytes: Option<bytes::Bytes> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        FileParserApiError::from_domain(DomainError::invalid_request(format!(
            "Multipart error: {}",
            e
        )))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            file_bytes = Some(field.bytes().await.map_err(|e| {
                FileParserApiError::from_domain(DomainError::io_error(format!(
                    "Failed to read file: {}",
                    e
                )))
            })?);
            break;
        }
    }

    let file_name = file_name.ok_or_else(|| {
        FileParserApiError::from_domain(DomainError::invalid_request(
            "No file field found in multipart request",
        ))
    })?;

    let file_bytes = file_bytes.ok_or_else(|| {
        FileParserApiError::from_domain(DomainError::invalid_request(
            "No file data found in multipart request",
        ))
    })?;

    info!(
        file_name = %file_name,
        size = file_bytes.len(),
        "Processing uploaded file"
    );

    let document = svc
        .parse_bytes(Some(&file_name), None, file_bytes)
        .await
        .map_err(FileParserApiError::from_domain)?;

    // Optionally render markdown
    let markdown = if render_md {
        Some(MarkdownRenderer::render(&document))
    } else {
        None
    };

    let response = FileParseResponseDto {
        document: ParsedDocumentDto::from(document),
        markdown,
    };

    Ok(Json(response))
}

/// Parse a file from a URL
#[tracing::instrument(
    name = "file_parser.parse_url",
    skip(svc, req_body, _ctx, query),
    fields(
        url = %req_body.url,
        render_markdown = ?query.render_markdown,
        request_id = Empty
    )
)]
#[axum::debug_handler]
pub async fn parse_url(
    Authz(_ctx): Authz,
    Extension(svc): Extension<std::sync::Arc<FileParserService>>,
    Query(query): Query<RenderMarkdownQuery>,
    Json(req_body): Json<ParseUrlRequest>,
) -> FileParserResult<JsonBody<FileParseResponseDto>> {
    let render_md = query.render_markdown.unwrap_or(false);

    info!(
        url = %req_body.url,
        render_markdown = render_md,
        "Parsing file from URL"
    );

    let url = url::Url::parse(&req_body.url)
        .map_err(|_| FileParserApiError::from_domain(DomainError::invalid_url(req_body.url)))?;

    let document = svc
        .parse_url(&url)
        .await
        .map_err(FileParserApiError::from_domain)?;

    // Optionally render markdown
    let markdown = if render_md {
        Some(MarkdownRenderer::render(&document))
    } else {
        None
    };

    let response = FileParseResponseDto {
        document: ParsedDocumentDto::from(document),
        markdown,
    };

    Ok(Json(response))
}
