use modkit::api::problem::Problem;

use crate::domain::error::DomainError;

/// Convert domain errors to HTTP Problem responses
pub fn domain_error_to_problem(err: DomainError) -> Problem {
    match err {
        DomainError::FileNotFound { path } => {
            Problem::new(404, "File Not Found", format!("File not found: {}", path))
        }

        DomainError::UnsupportedFileType { extension } => Problem::new(
            400,
            "Unsupported File Type",
            format!("Unsupported file type: {}", extension),
        ),

        DomainError::NoParserAvailable { extension } => Problem::new(
            415,
            "No Parser Available",
            format!("No parser available for extension: {}", extension),
        ),

        DomainError::ParseError { message } => Problem::new(422, "Parse Error", message),

        DomainError::IoError { message } => Problem::new(500, "IO Error", message),

        DomainError::InvalidUrl { url } => {
            Problem::new(400, "Invalid URL", format!("Invalid URL: {}", url))
        }

        DomainError::DownloadError { message } => Problem::new(502, "Download Error", message),

        DomainError::InvalidRequest { message } => Problem::new(400, "Invalid Request", message),
    }
}

/// Implement From<DomainError> for Problem so it works with ApiError
impl From<DomainError> for Problem {
    fn from(e: DomainError) -> Self {
        domain_error_to_problem(e)
    }
}
