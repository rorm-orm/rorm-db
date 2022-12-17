//! This module defines a wrapper for sqlx's Executor
//!
//! Unlike sqlx's Executor which provides several separate methods for different querying strategies,
//! our [Executor] has a single method which is generic using the [QueryStrategy] trait.

use rorm_sql::value::Value;

use crate::internal;

/// [QueryStrategy] returning nothing
///
/// `type Result<'result> = impl Future<Output = Result<(), Error>>`
pub struct Nothing;
impl QueryStrategy for Nothing {}

/// [QueryStrategy] returning a single row
///
/// `type Result<'result> = impl Future<Output = Result<Row, Error>>`
pub struct One;
impl QueryStrategy for One {}

/// [QueryStrategy] returning an optional row
///
/// `type Result<'result> = impl Future<Output = Result<Option<Row>, Error>>`
pub struct Optional;
impl QueryStrategy for Optional {}

/// [QueryStrategy] returning a vector of rows
///
/// `type Result<'result> = impl Future<Output = Result<Vec<Row>, Error>>`
pub struct All;
impl QueryStrategy for All {}

/// [QueryStrategy] returning a stream of rows
///
/// `type Result<'result> = impl Stream<Item = Result<Row, Error>>`
pub struct Stream;
impl QueryStrategy for Stream {}

/// Define how a query is send to and results retrieved from the database.
///
/// This trait is implemented on the following unit structs:
/// - [Nothing] retrieves nothing
/// - [Optional] retrieves an optional row
/// - [One] retrieves a single row
/// - [Stream] retrieves many rows in a stream
/// - [All] retrieves many rows in a vector
///
/// This trait has an associated `Result<'result>` type which is returned by [Executor::execute].
/// To avoid boxing, these types are quite big.
///
/// Each of those unit structs' docs (follow links above) contains an easy to read `impl Trait` version of the actual types.
pub trait QueryStrategy: internal::executor::QueryStrategyImpl {}

/// Some kind of database connection which can execute queries
///
/// This trait is implemented by the database connection itself as well as transactions.
pub trait Executor<'executor> {
    /// Execute a query
    ///
    /// ```skipped
    /// db.execute::<All>("SELECT * FROM foo;".to_string(), vec![]);
    /// ```
    fn execute<'data, 'result, Q>(
        self,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Q::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        Q: QueryStrategy;
}
