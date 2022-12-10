use std::marker::PhantomData;

use crate::transaction::Transaction;
use crate::Error;

use super::{no_sqlx, NotInstantiable};

pub(crate) type Impl<'db> = (PhantomData<&'db ()>, NotInstantiable);

/// Implementation of [Transaction::commit]
pub(crate) async fn commit(transaction: Transaction<'_>) -> Result<(), Error> {
    let _ = transaction.tx;
    no_sqlx();
}

/// Implementation of [Transaction::rollback]
pub(crate) async fn rollback(_transaction: Transaction<'_>) -> Result<(), Error> {
    no_sqlx();
}
