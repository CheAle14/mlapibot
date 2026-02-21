use std::{
    backtrace::{Backtrace, BacktraceStatus},
    error::Error,
};

use tokio_postgres::error::SqlState;

pub struct DbError {
    inner: tokio_postgres::Error,
    backtrace: Backtrace,
}

impl DbError {
    pub(crate) fn new(inner: tokio_postgres::Error) -> Self {
        Self {
            inner,
            backtrace: Backtrace::capture(),
        }
    }

    pub fn error_code(&self) -> Option<&SqlState> {
        self.inner.code()
    }

    pub fn is_table_undefined_err(&self) -> bool {
        self.error_code()
            .is_some_and(|err| err == &SqlState::UNDEFINED_TABLE)
    }
}

impl From<tokio_postgres::Error> for DbError {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)?;

        if let Some(source) = self.inner.source() {
            writeln!(f, "Caused by:")?;
            for (i, e) in std::iter::successors(Some(source), |&e| e.source()).enumerate() {
                writeln!(f, "\n- {i}: {e}")?;
            }
        }

        if self.backtrace.status() == BacktraceStatus::Captured {
            writeln!(f, "\nBacktrace: {}", self.backtrace)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

pub type DbResult<T> = std::result::Result<T, DbError>;
