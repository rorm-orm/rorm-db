//! This module holds the definition of transactions

use crate::{internal, Error};

/**
Transactions can be used to provide a safe way to execute multiple SQL operations
after another with a way to go back to the start without something changed in the
database.

Can be obtained using [`Database::start_transaction`](crate::Database::start_transaction).
*/
#[must_use = "A transaction needs to be committed."]
pub struct Transaction(pub(crate) internal::transaction::Impl);

impl Transaction {
    /// This function commits the transaction.
    pub async fn commit(self) -> Result<(), Error> {
        internal::transaction::commit(self).await
    }

    /// Use this function to abort the transaction.
    pub async fn rollback(self) -> Result<(), Error> {
        internal::transaction::rollback(self).await
    }
}

/// Either an owned or borrowed [`Transaction`].
///
/// "Guarding" a piece of code which has to be run in an transaction
/// (see [`Executor::ensure_transaction`](crate::executor::Executor::ensure_transaction))
#[must_use = "The potentially owned transaction needs to be committed."]
pub enum TransactionGuard<'tr> {
    /// An owned transaction
    Owned(Transaction),

    /// A borrowed transaction
    Borrowed(&'tr mut Transaction),
}

impl TransactionGuard<'_> {
    /// Get a reference to the guarded transaction
    pub fn get_transaction(&mut self) -> &mut Transaction {
        match self {
            TransactionGuard::Owned(tr) => tr,
            TransactionGuard::Borrowed(tr) => tr,
        }
    }

    /// Consume the guard, committing the potentially owned transaction.
    pub async fn commit(self) -> Result<(), Error> {
        if let TransactionGuard::Owned(tr) = self {
            tr.commit().await
        } else {
            Ok(())
        }
    }
}
