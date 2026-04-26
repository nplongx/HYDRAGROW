use thiserror::Error;

pub mod influx;
pub mod postgres; // Đổi từ sqlite -> postgres

#[derive(Error, Debug)]
pub enum DbError {
    #[error("PostgreSQL query failed: {0}")]
    PostgresError(#[from] sqlx::Error), // Đổi tên từ SqliteError

    #[error("InfluxDB operation failed: {0}")]
    InfluxError(#[from] influxdb2::BuildError),

    #[error("Record not found for device: {0}")]
    NotFound(String),

    #[error("Data parsing error: {0}")]
    ParseError(String),
}

pub type DbResult<T> = Result<T, DbError>;
