use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Api(String),
    Auth(String),
    Config(String),
    Io(std::io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Api(msg) => write!(f, "API error: {}", msg),
            AppError::Auth(msg) => write!(f, "Auth error: {}", msg),
            AppError::Config(msg) => write!(f, "Config error: {}", msg),
            AppError::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}
