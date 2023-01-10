use crate::transaction::Transaction;
use crate::Error;

pub(crate) type Impl = sqlx::Transaction<'static, sqlx::Any>;

/// Implementation of [Transaction::commit]
pub(crate) async fn commit(transaction: Transaction) -> Result<(), Error> {
    transaction.tx.commit().await.map_err(Error::SqlxError)
}

/// Implementation of [Transaction::rollback]
pub(crate) async fn rollback(transaction: Transaction) -> Result<(), Error> {
    transaction.tx.rollback().await.map_err(Error::SqlxError)
}
