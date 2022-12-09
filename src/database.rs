/*!
This module defines the main API wrapper.
*/

use std::future::Future;
use std::task::{Context, Poll};
use std::time::Duration;

use aliasable::string::AliasableString;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::StreamExt;
use log::{debug, LevelFilter};
use rorm_declaration::config::DatabaseDriver;
use rorm_sql::delete::Delete;
use rorm_sql::insert::Insert;
use rorm_sql::join_table::{JoinTableData, JoinTableImpl};
use rorm_sql::limit_clause::LimitClause;
use rorm_sql::ordering::OrderByEntry;
use rorm_sql::select::Select;
use rorm_sql::select_column::{SelectColumnData, SelectColumnImpl};
use rorm_sql::update::Update;
use rorm_sql::value::Value;
use rorm_sql::{conditional, value, DBImpl};
use sqlx::any::AnyPoolOptions;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;

use crate::error::Error;
use crate::result::QueryStream;
use crate::row::Row;
use crate::transaction::Transaction;
use crate::utils;

/**
Type alias for [SelectColumnData]..

As all databases use currently the same fields, a type alias is sufficient.
*/
pub type ColumnSelector<'a> = SelectColumnData<'a>;

/**
Type alias for [JoinTableData].

As all databases use currently the same fields, a type alias is sufficient.
*/
pub type JoinTable<'until_build, 'post_build> = JoinTableData<'until_build, 'post_build>;

/**
Configuration to create a database connection.

`min_connections` and `max_connections` must be greater than 0
and `max_connections` must be greater or equals `min_connections`.
 */
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DatabaseConfiguration {
    /// The driver and its corresponding settings
    pub driver: DatabaseDriver,
    /// Minimal connections to initialize upfront.
    pub min_connections: u32,
    /// Maximum connections that allowed to be created.
    pub max_connections: u32,
    /// If set to true, logging will be completely disabled.
    ///
    /// In case of None, false will be used.
    pub disable_logging: Option<bool>,
    /// Set the log level of SQL statements
    ///
    /// In case of None, [LevelFilter::Debug] will be used.
    pub statement_log_level: Option<LevelFilter>,
    /// Log level in case of slow statements (>300 ms)
    ///
    /// In case of None, [LevelFilter::Warn] will be used.
    pub slow_statement_log_level: Option<LevelFilter>,
}

impl DatabaseConfiguration {
    /**
    Create a new database configuration with some defaults set.

    **Defaults**:
    - `min_connections`: 1
    - `max_connections`: 10
    - `disable_logging`: None
    - `statement_log_level`: [Some] of [LevelFilter::Debug]
    - `slow_statement_log_level`: [Some] of [LevelFilter::Warn]

    **Parameter**:
    - `driver`: [DatabaseDriver]: Configuration of the database driver.
    */
    pub fn new(driver: DatabaseDriver) -> Self {
        DatabaseConfiguration {
            driver,
            min_connections: 1,
            max_connections: 10,
            disable_logging: None,
            statement_log_level: Some(LevelFilter::Debug),
            slow_statement_log_level: Some(LevelFilter::Warn),
        }
    }
}

/**
Main API wrapper.

All operations can be started with methods of this struct.
 */
#[derive(Clone)]
pub struct Database {
    pool: sqlx::Pool<sqlx::Any>,
    db_impl: DBImpl,
}

/**
All statements that take longer to execute than this value are considered
as slow statements.
*/
const SLOW_STATEMENTS: Duration = Duration::from_millis(300);

type Offset = u64;
type AnyQuery<'q> = sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>;
type AnyDb = sqlx::Any;

trait QueryType {
    type LimitArgument;
    type Future<'result>;

    fn limit_clause(argument: Self::LimitArgument) -> Option<LimitClause>;

    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: sqlx::Executor<'post_query, Database = AnyDb>;
}

trait QueryByFuture {
    type LimitArgument;
    type Sqlx;
    type Rorm;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: sqlx::Executor<'db, Database = AnyDb>;

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error>;

    fn convert_limit(argument: Self::LimitArgument) -> Option<LimitClause>;
}
impl<Q: QueryByFuture> QueryType for Q {
    type LimitArgument = Q::LimitArgument;
    type Future<'result> = QueryFuture<'result, Q::Sqlx, Q::Rorm>;

    fn limit_clause(argument: Self::LimitArgument) -> Option<LimitClause> {
        Q::convert_limit(argument)
    }

    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: sqlx::Executor<'post_query, Database = AnyDb>,
    {
        let query_string = AliasableString::from_unique(query);
        let query: &str = &query_string;
        let mut query = sqlx::query(unsafe { std::mem::transmute(query) });
        for x in values {
            query = utils::bind_param(query, x);
        }
        QueryFuture {
            query_string,
            original: Q::create_future(executor, query),
            map: Q::convert_result,
        }
    }
}
// DO NOT modify this struct without careful thought!!!
struct QueryFuture<'q, Sqlx, Rorm> {
    #[allow(dead_code)] // it's not "dead", it is aliased before landing in this struct
    query_string: AliasableString,
    original: BoxFuture<'q, Result<Sqlx, sqlx::Error>>,
    map: fn(Result<Sqlx, sqlx::Error>) -> Result<Rorm, Error>,
}
impl<'q, Sqlx, Rorm> Future for QueryFuture<'q, Sqlx, Rorm> {
    type Output = Result<Rorm, Error>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.original.as_mut().poll(cx) {
            Poll::Ready(output) => Poll::Ready((self.map)(output)),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct One;
impl QueryByFuture for One {
    type LimitArgument = Option<Offset>;
    type Sqlx = sqlx::any::AnyRow;
    type Rorm = Row;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: sqlx::Executor<'db, Database = AnyDb>,
    {
        executor.fetch_one(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result.map(Row::from).map_err(Error::SqlxError)
    }

    fn convert_limit(offset: Self::LimitArgument) -> Option<LimitClause> {
        Some(LimitClause { limit: 1, offset })
    }
}

struct Optional;
impl QueryByFuture for Optional {
    type LimitArgument = Option<Offset>;
    type Sqlx = Option<sqlx::any::AnyRow>;
    type Rorm = Option<Row>;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: sqlx::Executor<'db, Database = AnyDb>,
    {
        executor.fetch_optional(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result
            .map(|option| option.map(Row::from))
            .map_err(Error::SqlxError)
    }

    fn convert_limit(offset: Self::LimitArgument) -> Option<LimitClause> {
        Some(LimitClause { limit: 1, offset })
    }
}

struct All;
impl QueryByFuture for All {
    type LimitArgument = Option<LimitClause>;
    type Sqlx = Vec<sqlx::any::AnyRow>;
    type Rorm = Vec<Row>;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: sqlx::Executor<'db, Database = AnyDb>,
    {
        executor.fetch_all(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result
            .map(|vector| vector.into_iter().map(Row::from).collect())
            .map_err(Error::SqlxError)
    }

    fn convert_limit(argument: Self::LimitArgument) -> Option<LimitClause> {
        argument
    }
}

struct Stream;
impl QueryType for Stream {
    type LimitArgument = Option<LimitClause>;
    type Future<'result> = BoxStream<'result, Result<Row, Error>>;

    fn limit_clause(argument: Self::LimitArgument) -> Option<LimitClause> {
        argument
    }

    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: sqlx::Executor<'post_query, Database = AnyDb>,
    {
        QueryStream::build(query, values, executor).boxed()
    }
}

impl Database {
    /**
    Access the used driver at runtime.

    This can be used to generate SQL statements for the chosen dialect.
    */
    pub fn get_sql_dialect(&self) -> DBImpl {
        self.db_impl
    }

    /**
    Connect to the database using `configuration`.
     */
    pub async fn connect(configuration: DatabaseConfiguration) -> Result<Self, Error> {
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

    fn query<'result, 'db: 'result, 'post_query: 'result, F: QueryType>(
        &'db self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, 'post_query>],
        conditions: Option<&conditional::Condition<'post_query>>,
        order_by_clause: &[OrderByEntry<'_>],
        limit: F::LimitArgument,
        transaction: Option<&'db mut Transaction<'_>>,
    ) -> F::Future<'result> {
        let columns: Vec<SelectColumnImpl> = columns
            .iter()
            .map(|c| {
                self.db_impl.select_column(
                    c.table_name,
                    c.column_name,
                    c.select_alias,
                    c.aggregation,
                )
            })
            .collect();
        let joins: Vec<JoinTableImpl> = joins
            .iter()
            .map(|j| {
                self.db_impl
                    .join_table(j.join_type, j.table_name, j.join_alias, j.join_condition)
            })
            .collect();
        let mut q = self
            .db_impl
            .select(&columns, model, &joins, order_by_clause);

        if conditions.is_some() {
            q = q.where_clause(conditions.unwrap());
        }

        if let Some(limit) = F::limit_clause(limit) {
            q = q.limit_clause(limit);
        }

        let (query_string, bind_params) = q.build();

        debug!("SQL: {}", query_string);

        match transaction {
            None => F::query(&self.pool, query_string, bind_params),
            Some(transaction) => F::query(&mut transaction.tx, query_string, bind_params),
        }
    }

    /**
    This method is used to retrieve a stream of rows that matched the applied conditions.

    **Parameter**:
    - `model`: Name of the table.
    - `columns`: Columns to retrieve values from.
    - `joins`: Join tables expressions.
    - `conditions`: Optional conditions to apply.
    - `limit`: Optional limit / offset to apply to the query.
    - `transaction`: Optional transaction to execute the query on.
     */
    #[allow(clippy::too_many_arguments)]
    pub fn query_stream<'db, 'post_query, 'stream>(
        &'db self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, 'post_query>],
        conditions: Option<&conditional::Condition<'post_query>>,
        order_by_clause: &[OrderByEntry<'_>],
        limit: Option<LimitClause>,
        transaction: Option<&'stream mut Transaction<'_>>,
    ) -> BoxStream<'stream, Result<Row, Error>>
    where
        'post_query: 'stream,
        'db: 'stream,
    {
        self.query::<Stream>(
            model,
            columns,
            joins,
            conditions,
            order_by_clause,
            limit,
            transaction,
        )
    }

    /**
    This method is used to retrieve exactly one row from the table.
    An error is returned if no value could be retrieved.

    **Parameter**:
    - `model`: Model to query.
    - `columns`: Columns to retrieve values from.
    - `joins`: Join tables expressions.
    - `conditions`: Optional conditions to apply.
    - `offset`: Optional offset to apply to the query.
    - `transaction`: Optional transaction to execute the query on.
     */
    #[allow(clippy::too_many_arguments)]
    pub async fn query_one(
        &self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, '_>],
        conditions: Option<&conditional::Condition<'_>>,
        order_by_clause: &[OrderByEntry<'_>],
        offset: Option<u64>,
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<Row, Error> {
        self.query::<One>(
            model,
            columns,
            joins,
            conditions,
            order_by_clause,
            offset,
            transaction,
        )
        .await
    }

    /**
    This method is used to retrieve an optional row from the table.

    **Parameter**:
    - `model`: Model to query.
    - `columns`: Columns to retrieve values from.
    - `joins`: Join tables expressions.
    - `conditions`: Optional conditions to apply.
    - `offset`: Optional offset to apply to the query.
    - `transaction`: Optional transaction to execute the query on.
     */
    #[allow(clippy::too_many_arguments)]
    pub async fn query_optional(
        &self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, '_>],
        conditions: Option<&conditional::Condition<'_>>,
        order_by_clause: &[OrderByEntry<'_>],
        offset: Option<u64>,
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<Option<Row>, Error> {
        self.query::<Optional>(
            model,
            columns,
            joins,
            conditions,
            order_by_clause,
            offset,
            transaction,
        )
        .await
    }

    /**
    This method is used to retrieve all rows that match the provided query.

    **Parameter**:
    - `model`: Model to query.
    - `columns`: Columns to retrieve values from.
    - `joins`: Join tables expressions.
    - `conditions`: Optional conditions to apply.
    - `limit`: Optional limit / offset to apply to the query.
    - `transaction`: Optional transaction to execute the query on.
     */
    #[allow(clippy::too_many_arguments)]
    pub async fn query_all(
        &self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, '_>],
        conditions: Option<&conditional::Condition<'_>>,
        order_by_clause: &[OrderByEntry<'_>],
        limit: Option<LimitClause>,
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<Vec<Row>, Error> {
        self.query::<All>(
            model,
            columns,
            joins,
            conditions,
            order_by_clause,
            limit,
            transaction,
        )
        .await
    }

    /**
    This method is used to insert into a table.

    **Parameter**:
    - `model`: Table to insert to
    - `columns`: Columns to set `values` for.
    - `values`: Values to bind to the corresponding columns.
    - `transaction`: Optional transaction to execute the query on.
     */
    pub async fn insert_returning(
        &self,
        model: &str,
        columns: &[&str],
        values: &[value::Value<'_>],
        transaction: Option<&mut Transaction<'_>>,
        returning: &[&str],
    ) -> Result<Row, Error> {
        let value_rows = &[values];
        let q = self
            .db_impl
            .insert(model, columns, value_rows, Some(returning));

        let (query_string, bind_params) = q.build();

        debug!("SQL: {}", query_string);

        let mut tmp = sqlx::query(query_string.as_str());
        for x in bind_params {
            tmp = utils::bind_param(tmp, x);
        }

        match transaction {
            None => match tmp.fetch_one(&self.pool).await {
                Ok(row) => Ok(Row::from(row)),
                Err(err) => Err(Error::SqlxError(err)),
            },
            Some(transaction) => match tmp.fetch_one(&mut transaction.tx).await {
                Ok(row) => Ok(Row::from(row)),
                Err(err) => Err(Error::SqlxError(err)),
            },
        }
    }

    /**
    This method is used to insert into a table.

    **Parameter**:
    - `model`: Table to insert to
    - `columns`: Columns to set `values` for.
    - `values`: Values to bind to the corresponding columns.
    - `transaction`: Optional transaction to execute the query on.
     */
    pub async fn insert(
        &self,
        model: &str,
        columns: &[&str],
        values: &[value::Value<'_>],
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<(), Error> {
        let value_rows = &[values];
        let q = self.db_impl.insert(model, columns, value_rows, None);

        let (query_string, bind_params) = q.build();

        debug!("SQL: {}", query_string);

        let mut tmp = sqlx::query(query_string.as_str());
        for x in bind_params {
            tmp = utils::bind_param(tmp, x);
        }

        match transaction {
            None => match tmp.execute(&self.pool).await {
                Ok(_) => Ok(()),
                Err(err) => Err(Error::SqlxError(err)),
            },
            Some(transaction) => match tmp.execute(&mut transaction.tx).await {
                Ok(_) => Ok(()),
                Err(err) => Err(Error::SqlxError(err)),
            },
        }
    }

    /**
    This method is used to bulk insert rows.

    If one insert statement fails, the complete operation will be rolled back.

    **Parameter**:
    - `model`: Table to insert to
    - `columns`: Columns to set `rows` for.
    - `rows`: List of values to bind to the corresponding columns.
    - `transaction`: Optional transaction to execute the query on.
     */
    pub async fn insert_bulk(
        &self,
        model: &str,
        columns: &[&str],
        rows: &[&[value::Value<'_>]],
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<(), Error> {
        match transaction {
            None => {
                let mut tx = self.pool.begin().await?;
                for chunk in rows.chunks(25) {
                    let mut insert = self.db_impl.insert(model, columns, chunk, None);
                    insert = insert.rollback_transaction();
                    let (insert_query, insert_params) = insert.build();

                    debug!("SQL: {}", insert_query);

                    let mut q = sqlx::query(insert_query.as_str());

                    for x in insert_params {
                        q = utils::bind_param(q, x);
                    }

                    q.execute(&mut tx).await?;
                }
                tx.commit().await.map_err(Error::SqlxError)
            }
            Some(transaction) => {
                for chunk in rows.chunks(25) {
                    let mut insert = self.db_impl.insert(model, columns, chunk, None);
                    insert = insert.rollback_transaction();
                    let (insert_query, insert_params) = insert.build();

                    debug!("SQL: {}", insert_query);

                    let mut q = sqlx::query(insert_query.as_str());

                    for x in insert_params {
                        q = utils::bind_param(q, x);
                    }

                    q.execute(&mut transaction.tx).await?;
                }
                Ok(())
            }
        }
    }

    /**
    This method is used to bulk insert rows.

    If one insert statement fails, the complete operation will be rolled back.

    **Parameter**:
    - `model`: Table to insert to
    - `columns`: Columns to set `rows` for.
    - `rows`: List of values to bind to the corresponding columns.
    - `transaction`: Optional transaction to execute the query on.
     */
    pub async fn insert_bulk_returning_all(
        &self,
        model: &str,
        columns: &[&str],
        rows: &[&[value::Value<'_>]],
        transaction: Option<&mut Transaction<'_>>,
        returning: &[&str],
    ) -> Result<Vec<Row>, Error> {
        let mut inserted = Vec::with_capacity(rows.len());
        match transaction {
            None => {
                let mut tx = self.pool.begin().await?;
                for chunk in rows.chunks(25) {
                    let mut insert = self.db_impl.insert(model, columns, chunk, Some(returning));
                    insert = insert.rollback_transaction();
                    let (insert_query, insert_params) = insert.build();

                    debug!("SQL: {}", insert_query);

                    let mut q = sqlx::query(insert_query.as_str());

                    for x in insert_params {
                        q = utils::bind_param(q, x);
                    }

                    let rows = q.fetch_all(&mut tx).await?.into_iter().map(Row::from);
                    inserted.extend(rows);
                }
                tx.commit().await?;
                Ok(inserted)
            }
            Some(transaction) => {
                for chunk in rows.chunks(25) {
                    let mut insert = self.db_impl.insert(model, columns, chunk, Some(returning));
                    insert = insert.rollback_transaction();
                    let (insert_query, insert_params) = insert.build();

                    debug!("SQL: {}", insert_query);

                    let mut q = sqlx::query(insert_query.as_str());

                    for x in insert_params {
                        q = utils::bind_param(q, x);
                    }

                    let rows = q
                        .fetch_all(&mut transaction.tx)
                        .await?
                        .into_iter()
                        .map(Row::from);
                    inserted.extend(rows);
                }
                Ok(inserted)
            }
        }
    }

    /**
    This method is used to delete rows from a table.

    **Parameter**:
    - `model`: Name of the model to delete rows from
    - `condition`: Optional condition to apply.
    - `transaction`: Optional transaction to execute the query on.

    **Returns** the rows affected of the delete statement. Note that this also includes
    relations, etc.
     */
    pub async fn delete<'post_build>(
        &self,
        model: &str,
        condition: Option<&conditional::Condition<'post_build>>,
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<u64, Error> {
        let mut q = self.db_impl.delete(model);
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
            None => match tmp.execute(&self.pool).await {
                Ok(qr) => Ok(qr.rows_affected()),
                Err(err) => Err(Error::SqlxError(err)),
            },
            Some(transaction) => match tmp.execute(&mut transaction.tx).await {
                Ok(qr) => Ok(qr.rows_affected()),
                Err(err) => Err(Error::SqlxError(err)),
            },
        }
    }

    /**
    This method is used to update rows in a table.

    **Parameter**:
    - `model`: Name of the model to update rows from
    - `updates`: A list of updates. An update is a tuple that consists of a list of columns to
    update as well as the value to set to the columns.
    - `condition`: Optional condition to apply.
    - `transaction`: Optional transaction to execute the query on.

    **Returns** the rows affected from the update statement. Note that this also includes
    relations, etc.
     */
    pub async fn update<'post_build>(
        &self,
        model: &str,
        updates: &[(&str, value::Value<'post_build>)],
        condition: Option<&conditional::Condition<'post_build>>,
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<u64, Error> {
        let mut stmt = self.db_impl.update(model);

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
                .execute(&self.pool)
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

    /**
    Execute raw SQL statements on the database.

    If possible, the statement is executed as prepared statement.

    To bind parameter, use ? as placeholder in SQLite and MySQL
    and $1, $2, $n in Postgres.

    **Parameter**:
    - `query_string`: Reference to a valid SQL query.
    - `bind_params`: Optional list of values to bind in the query.
    - `transaction`: Optional transaction to execute the query on.

    **Returns** a list of rows. If there are no values to retrieve, an empty
    list is returned.
    */
    pub async fn raw_sql<'a>(
        &self,
        query_string: &'a str,
        bind_params: Option<&[value::Value<'a>]>,
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
                .fetch_all(&self.pool)
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

    /**
    Entry point for a [Transaction].
    */
    pub async fn start_transaction(&self) -> Result<Transaction, Error> {
        let tx = self.pool.begin().await.map_err(Error::SqlxError)?;

        Ok(Transaction { tx })
    }
}
