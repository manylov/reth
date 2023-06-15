//! Common CLI utility functions.

use eyre::{Result, WrapErr};
use reth_db::{
    cursor::DbCursorRO,
    database::Database,
    table::Table,
    transaction::{DbTx, DbTxMut},
};
use reth_interfaces::p2p::{
    headers::client::{HeadersClient, HeadersRequest},
    priority::Priority,
};
use reth_primitives::{BlockHashOrNumber, ChainSpec, HeadersDirection, SealedHeader};
use std::{
    env::VarError,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::info;

/// Get a single header from network
pub async fn get_single_header<Client>(
    client: Client,
    id: BlockHashOrNumber,
) -> Result<SealedHeader>
where
    Client: HeadersClient,
{
    let request = HeadersRequest { direction: HeadersDirection::Rising, limit: 1, start: id };

    let (peer_id, response) =
        client.get_headers_with_priority(request, Priority::High).await?.split();

    if response.len() != 1 {
        client.report_bad_message(peer_id);
        eyre::bail!("Invalid number of headers received. Expected: 1. Received: {}", response.len())
    }

    let header = response.into_iter().next().unwrap().seal_slow();

    let valid = match id {
        BlockHashOrNumber::Hash(hash) => header.hash() == hash,
        BlockHashOrNumber::Number(number) => header.number == number,
    };

    if !valid {
        client.report_bad_message(peer_id);
        eyre::bail!(
            "Received invalid header. Received: {:?}. Expected: {:?}",
            header.num_hash(),
            id
        );
    }

    Ok(header)
}

/// Wrapper over DB that implements many useful DB queries.
pub struct DbTool<'a, DB: Database> {
    pub(crate) db: &'a DB,
    pub(crate) chain: Arc<ChainSpec>,
}

impl<'a, DB: Database> DbTool<'a, DB> {
    /// Takes a DB where the tables have already been created.
    pub(crate) fn new(db: &'a DB, chain: Arc<ChainSpec>) -> eyre::Result<Self> {
        Ok(Self { db, chain })
    }

    /// Grabs the contents of the table within a certain index range and places the
    /// entries into a [`HashMap`][std::collections::HashMap].
    pub fn list<T: Table>(
        &mut self,
        skip: usize,
        len: usize,
        reverse: bool,
    ) -> Result<Vec<(T::Key, T::Value)>> {
        let data = self.db.view(|tx| {
            let mut cursor = tx.cursor_read::<T>().expect("Was not able to obtain a cursor.");

            if reverse {
                cursor.walk_back(None)?.skip(skip).take(len).collect::<Result<_, _>>()
            } else {
                cursor.walk(None)?.skip(skip).take(len).collect::<Result<_, _>>()
            }
        })?;

        data.map_err(|e| eyre::eyre!(e))
    }

    /// Grabs the content of the table for the given key
    pub fn get<T: Table>(&mut self, key: T::Key) -> Result<Option<T::Value>> {
        self.db.view(|tx| tx.get::<T>(key))?.map_err(|e| eyre::eyre!(e))
    }

    /// Drops the database at the given path.
    pub fn drop(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        info!(target: "reth::cli", "Dropping database at {:?}", path);
        std::fs::remove_dir_all(path).wrap_err("Dropping the database failed")?;
        Ok(())
    }

    /// Drops the provided table from the database.
    pub fn drop_table<T: Table>(&mut self) -> Result<()> {
        self.db.update(|tx| tx.clear::<T>())??;
        Ok(())
    }
}

/// Parses a user-specified path with support for environment variables and common shorthands (e.g.
/// ~ for the user's home directory).
pub fn parse_path(value: &str) -> Result<PathBuf, shellexpand::LookupError<VarError>> {
    shellexpand::full(value).map(|path| PathBuf::from(path.into_owned()))
}
