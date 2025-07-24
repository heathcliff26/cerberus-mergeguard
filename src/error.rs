use std::fmt::Display;

/// Error type for the application, encapsulating various error scenarios
#[derive(Debug)]
pub enum Error {
    ReadPrivateKey(String, std::io::Error),
    EncodingKey(jsonwebtoken::errors::Error),
    #[allow(clippy::upper_case_acronyms)]
    JWT(jsonwebtoken::errors::Error),
    InvalidBearerToken(),
    CreateRequest(reqwest::Error),
    Send(reqwest::Error),
    NonOkStatus(String, reqwest::StatusCode),
    Parse(&'static str, Box<dyn std::error::Error>),
    ReceiveBody(reqwest::Error),
    Serve(std::io::Error),
    BindPort(Box<dyn std::error::Error>),
    ReadConfigFile(String, std::io::Error),
    ParseConfigFile(String, serde_yaml::Error),
    InvalidConfig(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReadPrivateKey(path, err) => {
                write!(f, "Failed to read private key '{path}': {err}")
            }
            Error::EncodingKey(err) => {
                write!(f, "Failed to create encoding key: {err}")
            }
            Error::JWT(err) => {
                write!(f, "Failed to create JWT token: {err}")
            }
            Error::InvalidBearerToken() => {
                write!(f, "Invalid bearer token provided.")
            }
            Error::CreateRequest(err) => {
                write!(f, "Failed to create request: {err}")
            }
            Error::Send(err) => {
                write!(f, "Failed to send request: {}", full_error_stack(err))
            }
            Error::NonOkStatus(url, status) => {
                write!(f, "Request to '{url}' failed with status: {status}")
            }
            Error::Parse(url, err) => {
                write!(f, "Failed to parse response from '{url}': {err}")
            }
            Error::ReceiveBody(err) => {
                write!(f, "Failed to read response body: {err}")
            }
            Error::Serve(err) => {
                write!(f, "Server error: {err}")
            }
            Error::BindPort(err) => {
                write!(f, "Failed to bind port: {err}")
            }
            Error::ReadConfigFile(path, err) => {
                write!(f, "Failed to read config file '{path}': {err}")
            }
            Error::ParseConfigFile(path, err) => {
                write!(f, "Failed to parse config file '{path}': {err}")
            }
            Error::InvalidConfig(msg) => {
                write!(f, "Invalid configuration: {msg}")
            }
        }
    }
}

impl std::error::Error for Error {}

fn full_error_stack(mut e: &dyn std::error::Error) -> String {
    let mut s = format!("{e}");
    while let Some(src) = e.source() {
        s.push_str(&format!(": {src}"));
        e = src;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display_read_private_key() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error = Error::ReadPrivateKey("/path/to/key".to_string(), io_error);
        let display_string = format!("{}", error);
        assert!(display_string.contains("Failed to read private key '/path/to/key'"));
        assert!(display_string.contains("file not found"));
    }

    #[test]
    fn test_error_display_invalid_bearer_token() {
        let error = Error::InvalidBearerToken();
        let display_string = format!("{}", error);
        assert_eq!(display_string, "Invalid bearer token provided.");
    }

    #[test]
    fn test_error_display_non_ok_status() {
        let error = Error::NonOkStatus(
            "https://api.github.com".to_string(),
            reqwest::StatusCode::NOT_FOUND,
        );
        let display_string = format!("{}", error);
        assert_eq!(
            display_string,
            "Request to 'https://api.github.com' failed with status: 404 Not Found"
        );
    }

    #[test]
    fn test_error_display_invalid_config() {
        let error = Error::InvalidConfig("missing required field");
        let display_string = format!("{}", error);
        assert_eq!(
            display_string,
            "Invalid configuration: missing required field"
        );
    }

    #[test]
    fn test_error_display_read_config_file() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let error = Error::ReadConfigFile("/etc/config.yaml".to_string(), io_error);
        let display_string = format!("{}", error);
        assert!(display_string.contains("Failed to read config file '/etc/config.yaml'"));
        assert!(display_string.contains("permission denied"));
    }

    #[test]
    fn test_error_display_bind_port() {
        let io_error = io::Error::new(io::ErrorKind::AddrInUse, "address already in use");
        let error = Error::BindPort(Box::new(io_error));
        let display_string = format!("{}", error);
        assert!(display_string.contains("Failed to bind port"));
        assert!(display_string.contains("address already in use"));
    }

    #[test]
    fn test_error_is_error_trait() {
        let error = Error::InvalidBearerToken();
        // Test that Error implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_full_error_stack() {
        let inner_error = io::Error::new(io::ErrorKind::NotFound, "inner error");
        let outer_error = Error::ReadPrivateKey("test".to_string(), inner_error);
        let stack = full_error_stack(&outer_error);
        assert!(stack.contains("Failed to read private key 'test'"));
        assert!(stack.contains("inner error"));
    }
}
