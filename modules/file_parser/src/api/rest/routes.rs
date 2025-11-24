use crate::api::rest::handlers;
use crate::domain::service::FileParserService;
use axum::{Extension, Router};
use modkit::api::{OpenApiRegistry, OperationBuilder};
use std::sync::Arc;

pub fn register_routes(
    mut router: Router,
    openapi: &dyn OpenApiRegistry,
    service: Arc<FileParserService>,
) -> anyhow::Result<Router> {
    // GET /file-parser/info - Get information about available file parsers
    router = OperationBuilder::get("/file-parser/info")
        .operation_id("file_parser.get_parser_info")
        .summary("Get information about available file parsers")
        .tag("File Parser")
        .require_auth("file_parser", "read")
        .handler(handlers::get_parser_info)
        .json_response_with_schema::<crate::api::rest::dto::FileParserInfoDto>(
            openapi,
            200,
            "Information about available parsers",
        )
        .problem_response(openapi, 401, "Unauthorized")
        .problem_response(openapi, 403, "Forbidden")
        .problem_response(openapi, 500, "Internal Server Error")
        .register(router, openapi);

    // POST /file-parser/parse-local - Parse a file from a local path
    router = OperationBuilder::post("/file-parser/parse-local")
        .operation_id("file_parser.parse_local")
        .summary("Parse a file from a local path")
        .tag("File Parser")
        .require_auth("file_parser", "write")
        .query_param_typed(
            "render_markdown",
            false,
            "Render Markdown output if true (optional, default false)",
            "boolean",
        )
        .json_request::<crate::api::rest::dto::ParseLocalRequest>(openapi, "Local file path")
        .allow_content_types(&["application/json"])
        .handler(handlers::parse_local)
        .json_response_with_schema::<crate::api::rest::dto::FileParseResponseDto>(
            openapi,
            200,
            "Parsed document with optional markdown",
        )
        .problem_response(openapi, 400, "Bad Request")
        .problem_response(openapi, 401, "Unauthorized")
        .problem_response(openapi, 403, "Forbidden")
        .problem_response(openapi, 404, "File Not Found")
        .problem_response(openapi, 415, "Unsupported Media Type")
        .problem_response(openapi, 422, "Unprocessable Entity")
        .problem_response(openapi, 500, "Internal Server Error")
        .register(router, openapi);

    // POST /file-parser/upload - Upload and parse a file
    router = OperationBuilder::post("/file-parser/upload")
        .operation_id("file_parser.upload")
        .summary("Upload and parse a file")
        .description("Accepts multipart/form-data file uploads with field name 'file'")
        .tag("File Parser")
        .require_auth("file_parser", "write")
        .query_param_typed(
            "render_markdown",
            false,
            "Render Markdown output if true (optional, default false)",
            "boolean",
        )
        .allow_content_types(&["multipart/form-data"])
        .handler(handlers::upload_and_parse)
        .json_response_with_schema::<crate::api::rest::dto::FileParseResponseDto>(
            openapi,
            200,
            "Parsed document with optional markdown",
        )
        .problem_response(openapi, 400, "Bad Request")
        .problem_response(openapi, 401, "Unauthorized")
        .problem_response(openapi, 403, "Forbidden")
        .problem_response(openapi, 415, "Unsupported Media Type")
        .problem_response(openapi, 422, "Unprocessable Entity")
        .problem_response(openapi, 500, "Internal Server Error")
        .register(router, openapi);

    // POST /file-parser/parse-url - Parse a file from a URL
    router = OperationBuilder::post("/file-parser/parse-url")
        .operation_id("file_parser.parse_url")
        .summary("Parse a file from a URL")
        .tag("File Parser")
        .require_auth("file_parser", "write")
        .query_param_typed(
            "render_markdown",
            false,
            "Render Markdown output if true (optional, default false)",
            "boolean",
        )
        .json_request::<crate::api::rest::dto::ParseUrlRequest>(openapi, "URL to file")
        .allow_content_types(&["application/json"])
        .handler(handlers::parse_url)
        .json_response_with_schema::<crate::api::rest::dto::FileParseResponseDto>(
            openapi,
            200,
            "Parsed document with optional markdown",
        )
        .problem_response(openapi, 400, "Bad Request")
        .problem_response(openapi, 401, "Unauthorized")
        .problem_response(openapi, 403, "Forbidden")
        .problem_response(openapi, 415, "Unsupported Media Type")
        .problem_response(openapi, 422, "Unprocessable Entity")
        .problem_response(openapi, 502, "Bad Gateway")
        .problem_response(openapi, 500, "Internal Server Error")
        .register(router, openapi);

    router = router.layer(Extension(service.clone()));

    Ok(router)
}
