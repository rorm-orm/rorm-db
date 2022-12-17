use rorm_sql::limit_clause::LimitClause;

use crate::executor::{All, One, Optional, Stream};

type Offset = u64;

pub(crate) trait GetLimitClause {
    type Input;

    fn get_limit_clause(input: Self::Input) -> Option<LimitClause>;
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
