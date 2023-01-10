/*!
This module defines the main API wrapper.
*/

use futures::stream::{BoxStream, StreamExt};
use log::{debug, LevelFilter};
use rorm_declaration::config::DatabaseDriver;
use rorm_sql::delete::Delete;
use rorm_sql::insert::Insert;
use rorm_sql::join_table::JoinTableData;
use rorm_sql::limit_clause::LimitClause;
use rorm_sql::ordering::OrderByEntry;
use rorm_sql::select::Select;
use rorm_sql::select_column::SelectColumnData;
use rorm_sql::update::Update;
use rorm_sql::value::Value;
use rorm_sql::{conditional, value, DBImpl};

use crate::error::Error;
use crate::executor::{AffectedRows, All, Executor, Nothing, One, Optional, QueryStrategy, Stream};
use crate::internal;
use crate::query_type::GetLimitClause;
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
    #[deprecated(
        note = "Will be removed to reduce number of methods to maintain\nUse `query::<Stream>` instead!"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn query_stream<'db, 'post_query, 'stream>(
        &'db self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, 'post_query>],
        conditions: Option<&conditional::Condition<'post_query>>,
        order_by_clause: &[OrderByEntry<'_>],
        limit: Option<LimitClause>,
        transaction: Option<&'stream mut Transaction>,
    ) -> BoxStream<'stream, Result<Row, Error>>
    where
        'post_query: 'stream,
        'db: 'stream,
    {
        Self::query::<Stream>(
            self,
            model,
            columns,
            joins,
            conditions,
            order_by_clause,
            limit,
            transaction,
        )
        .boxed()
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
    #[deprecated(
        note = "Will be removed to reduce number of methods to maintain\nUse `query::<One>` instead!"
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn query_one(
        &self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, '_>],
        conditions: Option<&conditional::Condition<'_>>,
        order_by_clause: &[OrderByEntry<'_>],
        offset: Option<u64>,
        transaction: Option<&mut Transaction>,
    ) -> Result<Row, Error> {
        Self::query::<One>(
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
    #[deprecated(
        note = "Will be removed to reduce number of methods to maintain\nUse `query::<Optional>` instead!"
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn query_optional(
        &self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, '_>],
        conditions: Option<&conditional::Condition<'_>>,
        order_by_clause: &[OrderByEntry<'_>],
        offset: Option<u64>,
        transaction: Option<&mut Transaction>,
    ) -> Result<Option<Row>, Error> {
        Self::query::<Optional>(
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
    #[deprecated(
        note = "Will be removed to reduce number of methods to maintain\nUse `query::<All>` instead!"
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn query_all(
        &self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, '_>],
        conditions: Option<&conditional::Condition<'_>>,
        order_by_clause: &[OrderByEntry<'_>],
        limit: Option<LimitClause>,
        transaction: Option<&mut Transaction>,
    ) -> Result<Vec<Row>, Error> {
        Self::query::<All>(
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
    This method is used to retrieve rows that match the provided query.

    It is generic over a [QueryStrategy] which specifies how and how many rows to query.

    **Parameter**:
    - `model`: Model to query.
    - `columns`: Columns to retrieve values from.
    - `joins`: Join tables expressions.
    - `conditions`: Optional conditions to apply.
    - `limit`: Optional limit / offset to apply to the query.
        Depending on the query strategy, this is either [LimitClause] (for [All] and [Stream])
        or a simple [u64] (for [One] and [Optional]).
    - `transaction`: Optional transaction to execute the query on.
     */
    #[allow(clippy::too_many_arguments)]
    pub fn query<'result, 'db: 'result, 'post_query: 'result, Q: QueryStrategy + GetLimitClause>(
        &'db self,
        model: &str,
        columns: &[ColumnSelector<'_>],
        joins: &[JoinTable<'_, 'post_query>],
        conditions: Option<&conditional::Condition<'post_query>>,
        order_by_clause: &[OrderByEntry<'_>],
        limit: Option<Q::LimitOrOffset>,
        transaction: Option<&'db mut Transaction>,
    ) -> Q::Result<'result> {
        let columns: Vec<_> = columns
            .iter()
            .map(|c| {
                self.dialect().select_column(
                    c.table_name,
                    c.column_name,
                    c.select_alias,
                    c.aggregation,
                )
            })
            .collect();
        let joins: Vec<_> = joins
            .iter()
            .map(|j| {
                self.dialect()
                    .join_table(j.join_type, j.table_name, j.join_alias, j.join_condition)
            })
            .collect();
        let mut q = self
            .dialect()
            .select(&columns, model, &joins, order_by_clause);

        if let Some(condition) = conditions {
            q = q.where_clause(condition);
        }

        if let Some(limit) = Q::get_limit_clause(limit) {
            q = q.limit_clause(limit);
        }

        let (query_string, bind_params) = q.build();

        debug!("SQL: {}", query_string);

        match transaction {
            None => self.execute::<Q>(query_string, bind_params),
            Some(transaction) => transaction.execute::<Q>(query_string, bind_params),
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
    pub async fn insert_returning(
        &self,
        model: &str,
        columns: &[&str],
        values: &[Value<'_>],
        transaction: Option<&mut Transaction>,
        returning: &[&str],
    ) -> Result<Row, Error> {
        Self::generic_insert::<One>(self, model, columns, values, transaction, Some(returning))
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
        values: &[Value<'_>],
        transaction: Option<&mut Transaction>,
    ) -> Result<(), Error> {
        Self::generic_insert::<Nothing>(self, model, columns, values, transaction, None).await
    }

    /// Generic implementation of:
    /// - [Database::insert]
    /// - [Database::insert_returning]
    pub(crate) fn generic_insert<'result, 'db: 'result, 'post_query: 'result, Q: QueryStrategy>(
        db: &'db Database,
        model: &str,
        columns: &[&str],
        values: &[Value<'post_query>],
        transaction: Option<&'db mut Transaction>,
        returning: Option<&[&str]>,
    ) -> Q::Result<'result> {
        let values = &[values];
        let q = db.dialect().insert(model, columns, values, returning);

        let (query_string, bind_params): (_, Vec<Value<'post_query>>) = q.build();

        debug!("SQL: {}", query_string);

        match transaction {
            None => db.execute::<Q>(query_string, bind_params),
            Some(transaction) => transaction.execute::<Q>(query_string, bind_params),
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
        rows: &[&[Value<'_>]],
        transaction: Option<&mut Transaction>,
    ) -> Result<(), Error> {
        return match transaction {
            None => {
                let mut transaction = self.start_transaction().await?;
                with_transaction(self, &mut transaction, model, columns, rows).await?;
                transaction.commit().await
            }
            Some(transaction) => {
                with_transaction(self, transaction, model, columns, rows).await?;
                Ok(())
            }
        };
        async fn with_transaction(
            db: &Database,
            tx: &mut Transaction,
            model: &str,
            columns: &[&str],
            rows: &[&[Value<'_>]],
        ) -> Result<(), Error> {
            for chunk in rows.chunks(25) {
                let mut insert = db.dialect().insert(model, columns, chunk, None);
                insert = insert.rollback_transaction();
                let (insert_query, insert_params) = insert.build();

                debug!("SQL: {}", insert_query);

                tx.execute::<Nothing>(insert_query, insert_params).await?;
            }
            Ok(())
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
    pub async fn insert_bulk_returning(
        &self,
        model: &str,
        columns: &[&str],
        rows: &[&[Value<'_>]],
        transaction: Option<&mut Transaction>,
        returning: &[&str],
    ) -> Result<Vec<Row>, Error> {
        return match transaction {
            None => {
                let mut transaction = self.start_transaction().await?;
                let result =
                    with_transaction(self, &mut transaction, model, columns, rows, returning).await;
                transaction.commit().await?;
                result
            }
            Some(transaction) => {
                with_transaction(self, transaction, model, columns, rows, returning).await
            }
        };
        async fn with_transaction(
            db: &Database,
            tx: &mut Transaction,
            model: &str,
            columns: &[&str],
            rows: &[&[Value<'_>]],
            returning: &[&str],
        ) -> Result<Vec<Row>, Error> {
            let mut inserted = Vec::with_capacity(rows.len());
            for chunk in rows.chunks(25) {
                let mut insert = db.dialect().insert(model, columns, chunk, Some(returning));
                insert = insert.rollback_transaction();
                let (insert_query, insert_params) = insert.build();

                debug!("SQL: {}", insert_query);

                inserted.extend(tx.execute::<All>(insert_query, insert_params).await?);
            }
            Ok(inserted)
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
        transaction: Option<&mut Transaction>,
    ) -> Result<u64, Error> {
        let mut q = self.dialect().delete(model);
        if condition.is_some() {
            q = q.where_clause(condition.unwrap());
        }

        let (query_string, bind_params) = q.build();

        debug!("SQL: {}", query_string);

        match transaction {
            None => {
                self.execute::<AffectedRows>(query_string, bind_params)
                    .await
            }
            Some(transaction) => {
                transaction
                    .execute::<AffectedRows>(query_string, bind_params)
                    .await
            }
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
        updates: &[(&str, Value<'post_build>)],
        condition: Option<&conditional::Condition<'post_build>>,
        transaction: Option<&mut Transaction>,
    ) -> Result<u64, Error> {
        let mut stmt = self.dialect().update(model);

        for (column, value) in updates {
            stmt = stmt.add_update(column, *value);
        }

        if let Some(cond) = condition {
            stmt = stmt.where_clause(cond);
        }

        let (query_string, bind_params) = stmt.build()?;
        debug!("SQL: {}", query_string);

        match transaction {
            None => {
                self.execute::<AffectedRows>(query_string, bind_params)
                    .await
            }
            Some(transaction) => {
                transaction
                    .execute::<AffectedRows>(query_string, bind_params)
                    .await
            }
        }
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
        transaction: Option<&mut Transaction>,
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
