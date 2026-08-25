use thiserror::Error;

pub mod device_ownership;
pub mod influx;
pub mod postgres;
pub mod recipes;
#[cfg(test)]
pub mod tests;
pub mod users;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("PostgreSQL query failed: {0}")]
    PostgresError(#[from] sqlx::Error),

    #[error("InfluxDB operation failed: {0}")]
    InfluxError(#[from] influxdb2::BuildError),

    #[error("Record not found for device: {0}")]
    NotFound(String),

    #[error("Data parsing error: {0}")]
    ParseError(String),
}

pub type DbResult<T> = Result<T, DbError>;
