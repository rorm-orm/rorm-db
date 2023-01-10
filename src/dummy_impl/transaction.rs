use crate::transaction::Transaction;
use crate::Error;

use super::{no_sqlx, NotInstantiable};

pub(crate) type Impl = NotInstantiable;

/// Implementation of [Transaction::commit]
pub(crate) async fn commit(transaction: Transaction<'_>) -> Result<(), Error> {
    // "Read" tx at least once
    let _ = transaction.tx;
    no_sqlx();
}

/// Implementation of [Transaction::rollback]
pub(crate) async fn rollback(_transaction: Transaction<'_>) -> Result<(), Error> {
    no_sqlx();
}
