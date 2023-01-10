//! This module holds the definition of transactions

use crate::{internal, Error};
use rorm_sql::DBImpl;

/**
Transactions can be used to provide a safe way to execute multiple SQL operations
after another with a way to go back to the start without something changed in the
database.

Can be obtained using [crate::Database::start_transaction].
*/
#[must_use = "A transaction needs to be committed."]
pub struct Transaction {
    pub(crate) tx: internal::transaction::Impl,
    pub(crate) db_impl: DBImpl,
}

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
