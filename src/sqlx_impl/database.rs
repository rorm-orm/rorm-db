use std::time::Duration;

use futures::TryStreamExt;
use log::{debug, LevelFilter};
use rorm_declaration::config::DatabaseDriver;
use rorm_sql::value::Value;
use sqlx::ConnectOptions;

use crate::database::{Database, DatabaseConfiguration};
use crate::error::Error;
use crate::internal::any::{AnyExecutor, AnyPool};
use crate::internal::utils;
use crate::row::Row;
use crate::transaction::Transaction;

pub(crate) type Impl = AnyPool;

/// All statements that take longer to execute than this value are considered
/// as slow statements.
const SLOW_STATEMENTS: Duration = Duration::from_millis(300);

/// Implementation of [Database::connect]
pub(crate) async fn connect(configuration: DatabaseConfiguration) -> Result<Database, Error> {
    if configuration.max_connections < configuration.min_connections {
        return Err(Error::ConfigurationError(String::from(
            "max_connections must not be less than min_connections",
        )));
    }

    if configuration.min_connections == 0 {
        return Err(Error::ConfigurationError(String::from(
            "min_connections must not be 0",
        )));
    }

    match &configuration.driver {
        DatabaseDriver::SQLite { filename, .. } => {
            if filename.is_empty() {
                return Err(Error::ConfigurationError(String::from(
                    "filename must not be empty",
                )));
            }
        }
        DatabaseDriver::Postgres { name, .. } => {
            if name.is_empty() {
                return Err(Error::ConfigurationError(String::from(
                    "name must not be empty",
                )));
            }
        }
        DatabaseDriver::MySQL { name, .. } => {
            if name.is_empty() {
                return Err(Error::ConfigurationError(String::from(
                    "name must not be empty",
                )));
            }
        }
    };

    macro_rules! pool_options {
        ($Pool:ty) => {
            <$Pool>::new()
                .min_connections(configuration.min_connections)
                .max_connections(configuration.max_connections)
        };
    }

    let slow_log_level = configuration
        .slow_statement_log_level
        .unwrap_or(LevelFilter::Warn);
    let log_level = configuration
        .statement_log_level
        .unwrap_or(LevelFilter::Debug);
    let disabled_logging = configuration.disable_logging.unwrap_or(false);

    let pool: Impl = match &configuration.driver {
        #[cfg(feature = "sqlite")]
        DatabaseDriver::SQLite { filename } => {
            let connect_options = sqlx::sqlite::SqliteConnectOptions::new()
                .create_if_missing(true)
                .filename(filename);
            let connect_options = if disabled_logging {
                connect_options.disable_statement_logging()
            } else {
                connect_options
                    .log_statements(log_level)
                    .log_slow_statements(slow_log_level, SLOW_STATEMENTS)
            };
            Impl::Sqlite(
                pool_options!(sqlx::sqlite::SqlitePoolOptions)
                    .connect_with(connect_options)
                    .await?,
            )
        }
        #[cfg(not(feature = "sqlite"))]
        DatabaseDriver::SQLite { .. } => {
            return Err(Error::ConfigurationError(
                "sqlite database is not enabled".to_string(),
            ));
        }
        #[cfg(feature = "postgres")]
        DatabaseDriver::Postgres {
            host,
            port,
            name,
            user,
            password,
        } => {
            let connect_options = sqlx::postgres::PgConnectOptions::new()
                .host(host.as_str())
                .port(*port)
                .username(user.as_str())
                .password(password.as_str())
                .database(name.as_str());
            let connect_options = if disabled_logging {
                connect_options.disable_statement_logging()
            } else {
                connect_options
                    .log_statements(log_level)
                    .log_slow_statements(slow_log_level, SLOW_STATEMENTS)
            };
            Impl::Postgres(
                pool_options!(sqlx::postgres::PgPoolOptions)
                    .connect_with(connect_options)
                    .await?,
            )
        }
        #[cfg(not(feature = "postgres"))]
        DatabaseDriver::Postgres { .. } => {
            return Err(Error::ConfigurationError(
                "postgres database is not enabled".to_string(),
            ));
        }
        #[cfg(feature = "mysql")]
        DatabaseDriver::MySQL {
            name,
            host,
            port,
            user,
            password,
        } => {
            let connect_options = sqlx::mysql::MySqlConnectOptions::new()
                .host(host.as_str())
                .port(*port)
                .username(user.as_str())
                .password(password.as_str())
                .database(name.as_str());
            let connect_options = if disabled_logging {
                connect_options.disable_statement_logging()
            } else {
                connect_options
                    .log_statements(log_level)
                    .log_slow_statements(slow_log_level, SLOW_STATEMENTS)
            };
            Impl::MySql(
                pool_options!(sqlx::mysql::MySqlPoolOptions)
                    .connect_with(connect_options)
                    .await?,
            )
        }
        #[cfg(not(feature = "mysql"))]
        DatabaseDriver::MySQL { .. } => {
            return Err(Error::ConfigurationError(
                "mysql database is not enabled".to_string(),
            ));
        }
    };

    Ok(Database(pool))
}

/// Implementation of [Database::raw_sql]
pub async fn raw_sql<'a>(
    db: &Database,
    query_string: &'a str,
    bind_params: Option<&[Value<'a>]>,
    transaction: Option<&mut Transaction>,
) -> Result<Vec<Row>, Error> {
    debug!("SQL: {}", query_string);

    let mut query = if let Some(transaction) = transaction {
        transaction.0.query(query_string)
    } else {
        db.0.query(query_string)
    };

    if let Some(params) = bind_params {
        for param in params {
            utils::bind_param(&mut query, *param);
        }
    }

    query
        .fetch_many()
        .try_filter_map(|either| async { Ok(either.right().map(Row)) })
        .err_into()
        .try_collect()
        .await
}

/// Implementation of [Database::start_transaction]
pub async fn start_transaction(db: &Database) -> Result<Transaction, Error> {
    Ok(Transaction(db.0.begin().await?))
}
