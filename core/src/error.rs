use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Usage(String),
    #[error("config: {0}")]
    Config(String),
    #[error("auth: {0}")]
    Auth(String),
    #[error("io: {0}")]
    Io(String),
    #[error("api: {0}")]
    Api(String),
    #[error("{0}")]
    Internal(String),
}

impl Error {
    /// sysexits-style process exit codes.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Usage(_) => 2,
            Error::Io(_) => 74,
            _ => 70,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_map_by_variant() {
        assert_eq!(Error::Usage("x".into()).exit_code(), 2);
        assert_eq!(Error::Io("x".into()).exit_code(), 74);
        assert_eq!(Error::Api("x".into()).exit_code(), 70);
    }
}
