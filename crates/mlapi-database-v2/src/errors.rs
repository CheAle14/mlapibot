use tokio_postgres::error::SqlState;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct DbError {
    #[from]
    inner: tokio_postgres::Error,
}

impl DbError {
    pub fn error_code(&self) -> Option<&SqlState> {
        self.inner.code()
    }

    pub fn is_table_undefined_err(&self) -> bool {
        self.error_code()
            .is_some_and(|err| err == &SqlState::UNDEFINED_TABLE)
    }
}

pub type DbResult<T> = std::result::Result<T, DbError>;
