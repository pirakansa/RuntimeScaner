use std::fmt::{Display, Formatter};
use std::io;

pub type Result<T> = std::result::Result<T, AuditError>;

#[derive(Debug)]
pub enum AuditError {
    Message(String),
    Io {
        context: String,
        source: io::Error,
    },
    CommandFailed {
        program: String,
        status: Option<i32>,
        stderr: String,
    },
}

impl AuditError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

impl Display for AuditError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => write!(formatter, "{message}"),
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::CommandFailed {
                program,
                status,
                stderr,
            } => {
                write!(
                    formatter,
                    "{program} failed with status {}",
                    status
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "terminated by signal".to_string())
                )?;
                if !stderr.trim().is_empty() {
                    write!(formatter, ": {}", stderr.trim())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for AuditError {}
