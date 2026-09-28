use std::error::Error;
use std::fmt;
use std::io;
use zip::result::ZipError;

#[derive(Debug)]
pub enum AppError {
    Network(reqwest::Error),
    Io(io::Error),
    Json(serde_json::Error),
    Zip(ZipError),
    Cancelled,
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Network(e) => write!(f, "網路錯誤: {}", e),
            AppError::Io(e) => write!(f, "檔案 I/O 錯誤: {}", e),
            AppError::Json(e) => write!(f, "JSON 解析錯誤: {}", e),
            AppError::Zip(e) => write!(f, "ZIP 解壓錯誤: {}", e),
            AppError::Cancelled => write!(f, "使用者已取消轉換"),
            AppError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::Network(error)
    }
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        AppError::Io(error)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Json(error)
    }
}

impl From<ZipError> for AppError {
    fn from(error: ZipError) -> Self {
        AppError::Zip(error)
    }
}