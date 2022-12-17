use std::future::Ready;

use futures::stream::Empty;

use crate::executor::{All, Nothing, One, Optional, Stream};
use crate::{Error, Row};

pub trait QueryStrategyImpl {
    type Result<'a>;
}

impl QueryStrategyImpl for Nothing {
    type Result<'result> = Ready<Result<(), Error>>;
}

impl QueryStrategyImpl for One {
    type Result<'result> = Ready<Result<Row, Error>>;
}

impl QueryStrategyImpl for Optional {
    type Result<'result> = Ready<Result<Option<Row>, Error>>;
}

impl QueryStrategyImpl for All {
    type Result<'result> = Ready<Result<Vec<Row>, Error>>;
}

impl QueryStrategyImpl for Stream {
    type Result<'result> = Empty<Result<Row, Error>>;
}
