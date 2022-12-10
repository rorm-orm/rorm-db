/*!
This module defines the main API wrapper.
*/

use futures::stream::BoxStream;
use log::LevelFilter;
use rorm_declaration::config::DatabaseDriver;
use rorm_sql::join_table::JoinTableData;
use rorm_sql::limit_clause::LimitClause;
use rorm_sql::ordering::OrderByEntry;
use rorm_sql::select_column::SelectColumnData;
use rorm_sql::{conditional, value, DBImpl};

use crate::error::Error;
use crate::internal;
use crate::query_type::{All, Nothing, One, Optional, Stream};
use crate::row::Row;
use crate::transaction::Transaction;

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
    pub(crate) pool: internal::database::Impl,
    pub(crate) db_impl: DBImpl,
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
        internal::database::connect(configuration).await
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
        internal::database::query::<Stream>(
            self,
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
        internal::database::query::<One>(
            self,
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
        internal::database::query::<Optional>(
            self,
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
        internal::database::query::<All>(
            self,
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
        internal::database::insert::<One>(
            self,
            model,
            columns,
            values,
            transaction,
            Some(returning),
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
    pub async fn insert(
        &self,
        model: &str,
        columns: &[&str],
        values: &[value::Value<'_>],
        transaction: Option<&mut Transaction<'_>>,
    ) -> Result<(), Error> {
        internal::database::insert::<Nothing>(self, model, columns, values, transaction, None).await
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
        internal::database::insert_bulk(self, model, columns, rows, transaction).await
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
    pub async fn insert_bulk_returning(
        &self,
        model: &str,
        columns: &[&str],
        rows: &[&[value::Value<'_>]],
        transaction: Option<&mut Transaction<'_>>,
        returning: &[&str],
    ) -> Result<Vec<Row>, Error> {
        internal::database::insert_bulk_returning(
            self,
            model,
            columns,
            rows,
            transaction,
            returning,
        )
        .await
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
        internal::database::delete(self, model, condition, transaction).await
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
        internal::database::update(self, model, updates, condition, transaction).await
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
        internal::database::raw_sql(self, query_string, bind_params, transaction).await
    }

    /**
    Entry point for a [Transaction].
    */
    pub async fn start_transaction(&self) -> Result<Transaction, Error> {
        internal::database::start_transaction(self).await
    }
}
