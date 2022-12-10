use crate::row::{Decode, RowIndex};
use crate::{Error, Row};

use super::{no_sqlx, NotInstantiable};

pub(crate) type Impl = NotInstantiable;

/// Implementation of [Row::get]
pub(crate) fn get<'r, T, I>(_row: &'r Row, _index: I) -> Result<T, Error>
where
    T: Decode<'r>,
    I: RowIndex,
{
    no_sqlx();
}
