use crate::transaction::Transaction;
use crate::Error;

pub(crate) type Impl<'db> = sqlx::Transaction<'db, sqlx::Any>;

/// Implementation of [Transaction::commit]
pub(crate) async fn commit(transaction: Transaction<'_>) -> Result<(), Error> {
    transaction.tx.commit().await.map_err(Error::SqlxError)
}

/// Implementation of [Transaction::rollback]
pub(crate) async fn rollback(transaction: Transaction<'_>) -> Result<(), Error> {
    transaction.tx.rollback().await.map_err(Error::SqlxError)
}
