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
        s.push_str(&format!(": {}", src));
        e = src;
    }
    s
}
