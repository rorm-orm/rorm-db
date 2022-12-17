use std::time::Duration;

use log::{debug, LevelFilter};
use rorm_declaration::config::DatabaseDriver;
use rorm_sql::delete::Delete;
use rorm_sql::insert::Insert;
use rorm_sql::update::Update;
use rorm_sql::value::Value;
use rorm_sql::{conditional, DBImpl};
use sqlx::any::AnyPoolOptions;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;

use crate::database::{Database, DatabaseConfiguration};
use crate::error::Error;
use crate::executor::{All, Executor, Nothing};
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

/// Implementation of [Database::insert_bulk]
pub async fn insert_bulk(
    db: &Database,
    model: &str,
    columns: &[&str],
    rows: &[&[Value<'_>]],
    transaction: Option<&mut Transaction<'_>>,
) -> Result<(), Error> {
    return match transaction {
        None => {
            let mut transaction = db.start_transaction().await?;
            with_transaction(db, &mut transaction, model, columns, rows).await?;
            transaction.commit().await
        }
        Some(transaction) => {
            with_transaction(db, transaction, model, columns, rows).await?;
            Ok(())
        }
    };
    async fn with_transaction(
        db: &Database,
        tx: &mut Transaction<'_>,
        model: &str,
        columns: &[&str],
        rows: &[&[Value<'_>]],
    ) -> Result<(), Error> {
        for chunk in rows.chunks(25) {
            let mut insert = db.db_impl.insert(model, columns, chunk, None);
            insert = insert.rollback_transaction();
            let (insert_query, insert_params) = insert.build();

            debug!("SQL: {}", insert_query);

            tx.execute::<Nothing>(insert_query, insert_params).await?;
        }
        Ok(())
    }
}

/// Implementation of [Database::insert_bulk_returning]
pub async fn insert_bulk_returning(
    db: &Database,
    model: &str,
    columns: &[&str],
    rows: &[&[Value<'_>]],
    transaction: Option<&mut Transaction<'_>>,
    returning: &[&str],
) -> Result<Vec<Row>, Error> {
    return match transaction {
        None => {
            let mut transaction = db.start_transaction().await?;
            let result =
                with_transaction(db, &mut transaction, model, columns, rows, returning).await;
            transaction.commit().await?;
            result
        }
        Some(transaction) => {
            with_transaction(db, transaction, model, columns, rows, returning).await
        }
    };
    async fn with_transaction(
        db: &Database,
        tx: &mut Transaction<'_>,
        model: &str,
        columns: &[&str],
        rows: &[&[Value<'_>]],
        returning: &[&str],
    ) -> Result<Vec<Row>, Error> {
        let mut inserted = Vec::with_capacity(rows.len());
        for chunk in rows.chunks(25) {
            let mut insert = db.db_impl.insert(model, columns, chunk, Some(returning));
            insert = insert.rollback_transaction();
            let (insert_query, insert_params) = insert.build();

            debug!("SQL: {}", insert_query);

            inserted.extend(tx.execute::<All>(insert_query, insert_params).await?);
        }
        Ok(inserted)
    }
}

/// Implementation of [Database::delete]
pub async fn delete<'post_build>(
    db: &Database,
    model: &str,
    condition: Option<&conditional::Condition<'post_build>>,
    transaction: Option<&mut Transaction<'_>>,
) -> Result<u64, Error> {
    let mut q = db.db_impl.delete(model);
    if condition.is_some() {
        q = q.where_clause(condition.unwrap());
    }

    let (query_string, bind_params) = q.build();

    debug!("SQL: {}", query_string);

    let mut tmp = sqlx::query(query_string.as_str());
    for x in bind_params {
        tmp = utils::bind_param(tmp, x);
    }

    match transaction {
        None => match tmp.execute(&db.pool).await {
            Ok(qr) => Ok(qr.rows_affected()),
            Err(err) => Err(Error::SqlxError(err)),
        },
        Some(transaction) => match tmp.execute(&mut transaction.tx).await {
            Ok(qr) => Ok(qr.rows_affected()),
            Err(err) => Err(Error::SqlxError(err)),
        },
    }
}

/// Implementation of [Database::update]
pub async fn update<'post_build>(
    db: &Database,
    model: &str,
    updates: &[(&str, Value<'post_build>)],
    condition: Option<&conditional::Condition<'post_build>>,
    transaction: Option<&mut Transaction<'_>>,
) -> Result<u64, Error> {
    let mut stmt = db.db_impl.update(model);

    for (column, value) in updates {
        stmt = stmt.add_update(column, *value);
    }

    if let Some(cond) = condition {
        stmt = stmt.where_clause(cond);
    }

    let (query_string, bind_params) = stmt.build()?;
    debug!("SQL: {}", query_string);

    let mut q = sqlx::query(&query_string);
    for x in bind_params {
        q = utils::bind_param(q, x);
    }

    Ok(match transaction {
        None => q
            .execute(&db.pool)
            .await
            .map_err(Error::SqlxError)?
            .rows_affected(),
        Some(transaction) => q
            .execute(&mut transaction.tx)
            .await
            .map_err(Error::SqlxError)?
            .rows_affected(),
    })
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
