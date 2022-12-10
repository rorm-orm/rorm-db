//! Abstraction over which concrete method of [Executor] to use

use rorm_sql::limit_clause::LimitClause;
use rorm_sql::value::Value;

pub(crate) struct Nothing;
pub(crate) struct One;
pub(crate) struct Optional;
pub(crate) struct All;
pub(crate) struct Stream;

type Offset = u64;

pub(crate) trait GetLimitClause {
    type Input;

    fn get_limit_clause(input: Self::Input) -> Option<LimitClause>;
}

pub(crate) trait QueryType {
    type Future<'result>;

    #[cfg(feature = "sqlx")]
    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: sqlx::Executor<'post_query, Database = sqlx::Any>;

    #[cfg(not(feature = "sqlx"))]
    fn query<E>(executor: E, query: String, values: Vec<Value>) -> Self::Future<'static>;
}

impl GetLimitClause for Stream {
    type Input = Option<LimitClause>;

    fn get_limit_clause(limit: Self::Input) -> Option<LimitClause> {
        limit
    }
}

impl GetLimitClause for All {
    type Input = Option<LimitClause>;

    fn get_limit_clause(limit: Self::Input) -> Option<LimitClause> {
        limit
    }
}

impl GetLimitClause for Optional {
    type Input = Option<Offset>;

    fn get_limit_clause(offset: Self::Input) -> Option<LimitClause> {
        Some(LimitClause { limit: 1, offset })
    }
}

impl GetLimitClause for One {
    type Input = Option<Offset>;

    fn get_limit_clause(offset: Self::Input) -> Option<LimitClause> {
        Some(LimitClause { limit: 1, offset })
    }
}
