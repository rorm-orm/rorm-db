use std::time::Duration;

use log::{debug, LevelFilter};
use rorm_declaration::config::DatabaseDriver;
use rorm_sql::value::Value;
use rorm_sql::DBImpl;
use sqlx::any::AnyPoolOptions;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;

use crate::database::{Database, DatabaseConfiguration};
use crate::error::Error;
use crate::row::Row;
use crate::transaction::Transaction;
use crate::utils;

pub(crate) type Impl = sqlx::Pool<sqlx::Any>;

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

    let database;
    let pool_options = AnyPoolOptions::new()
        .min_connections(configuration.min_connections)
        .max_connections(configuration.max_connections);

    let slow_log_level = configuration
        .slow_statement_log_level
        .unwrap_or(LevelFilter::Warn);
    let log_level = configuration
        .statement_log_level
        .unwrap_or(LevelFilter::Debug);
    let disabled_logging = configuration.disable_logging.unwrap_or(false);

    let pool: sqlx::Pool<sqlx::Any> = match &configuration.driver {
        DatabaseDriver::SQLite { filename } => {
            let mut connect_options = SqliteConnectOptions::new()
                .create_if_missing(true)
                .filename(filename);

            if disabled_logging {
                connect_options.disable_statement_logging();
            } else {
                connect_options.log_statements(log_level);
                connect_options.log_slow_statements(slow_log_level, SLOW_STATEMENTS);
            }

            pool_options.connect_with(connect_options.into()).await?
        }
        DatabaseDriver::Postgres {
            host,
            port,
            name,
            user,
            password,
        } => {
            let mut connect_options = PgConnectOptions::new()
                .host(host.as_str())
                .port(*port)
                .username(user.as_str())
                .password(password.as_str())
                .database(name.as_str());

            if disabled_logging {
                connect_options.disable_statement_logging();
            } else {
                connect_options.log_statements(log_level);
                connect_options.log_slow_statements(slow_log_level, SLOW_STATEMENTS);
            }

            pool_options.connect_with(connect_options.into()).await?
        }
        DatabaseDriver::MySQL {
            name,
            host,
            port,
            user,
            password,
        } => {
            let mut connect_options = MySqlConnectOptions::new()
                .host(host.as_str())
                .port(*port)
                .username(user.as_str())
                .password(password.as_str())
                .database(name.as_str());

            if disabled_logging {
                connect_options.disable_statement_logging();
            } else {
                connect_options.log_statements(log_level);
                connect_options.log_slow_statements(slow_log_level, SLOW_STATEMENTS);
            }

            pool_options.connect_with(connect_options.into()).await?
        }
    };

    database = Database {
        pool,
        db_impl: match &configuration.driver {
            DatabaseDriver::SQLite { .. } => DBImpl::SQLite,
            DatabaseDriver::Postgres { .. } => DBImpl::Postgres,
            DatabaseDriver::MySQL { .. } => DBImpl::MySQL,
        },
    };

    Ok(database)
}

/// Implementation of [Database::raw_sql]
pub async fn raw_sql<'a>(
    db: &Database,
    query_string: &'a str,
    bind_params: Option<&[Value<'a>]>,
    transaction: Option<&mut Transaction<'_>>,
) -> Result<Vec<Row>, Error> {
    debug!("SQL: {}", query_string);

    let mut q = sqlx::query(query_string);
    if let Some(params) = bind_params {
        for x in params {
            q = utils::bind_param(q, *x);
        }
    }

    match transaction {
        None => q
            .fetch_all(&db.pool)
            .await
            .map(|vector| vector.into_iter().map(Row::from).collect())
            .map_err(Error::SqlxError),
        Some(transaction) => q
            .fetch_all(&mut transaction.tx)
            .await
            .map(|vector| vector.into_iter().map(Row::from).collect())
            .map_err(Error::SqlxError),
    }
}

/// Implementation of [Database::start_transaction]
pub async fn start_transaction(db: &Database) -> Result<Transaction, Error> {
    let tx = db.pool.begin().await.map_err(Error::SqlxError)?;

    Ok(Transaction { tx })
}
