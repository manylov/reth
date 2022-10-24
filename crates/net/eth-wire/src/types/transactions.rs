//! Implements the `GetPooledTransactions` and `PooledTransactions` message types.
use reth_primitives::{TransactionSigned, H256};
use reth_rlp::{RlpDecodableWrapper, RlpEncodableWrapper};
use thiserror::Error;

/// A list of transaction hashes that the peer would like transaction bodies for.
#[derive(Clone, Debug, PartialEq, Eq, RlpEncodableWrapper, RlpDecodableWrapper)]
pub struct GetPooledTransactions(
    /// The transaction hashes to request transaction bodies for.
    pub Vec<H256>,
);

impl<T> From<Vec<T>> for GetPooledTransactions
where
    T: Into<H256>,
{
    fn from(hashes: Vec<T>) -> Self {
        GetPooledTransactions(hashes.into_iter().map(|h| h.into()).collect())
    }
}

/// The response to [`GetPooledTransactions`], containing the transaction bodies associated with
/// the requested hashes.
///
/// This response may not contain all bodies requested, but the bodies should be in the same order
/// as the request's hashes. Hashes may be skipped, and the client should ensure that each body
/// corresponds to a requested hash. Hashes may need to be re-requested if the bodies are not
/// included in the response.
#[derive(Clone, Debug, PartialEq, Eq, RlpEncodableWrapper, RlpDecodableWrapper)]
pub struct PooledTransactions(
    /// The transaction bodies, each of which should correspond to a requested hash.
    pub Vec<TransactionSigned>,
);

/// An error that may occur while matching a [`GetPooledTransactions`] request to a
/// [`PooledTransactions`] response.
#[derive(Debug, Error)]
pub enum PooledTransactionsError {
    /// Thrown if there are transactions that do not match a requested hash.
    #[error("one or more transactions do not match a requested hash")]
    UnmatchedTransactions,
}

impl PooledTransactions {
    /// Given a list of hashes, split the hashes into those that match a transaction in the
    /// response, and those that do not.
    /// Assumes the transactions are in the same order as the request's hashes.
    pub fn split_transactions_by_hashes<T: Clone + Into<H256>>(
        &self,
        hashes: Vec<T>,
    ) -> Result<(Vec<H256>, Vec<H256>), PooledTransactionsError> {
        // we need to loop through each transaction, skipping over hashes that we don't have a
        // transaction for
        let mut missing_hashes = Vec::new();
        let mut hash_iter = hashes.iter();
        let (matched_transactions, unmatched_transactions): (
            Vec<&TransactionSigned>,
            Vec<&TransactionSigned>,
        ) = self.0.iter().partition(|tx| {
            for hash in &mut hash_iter {
                let curr_hash = hash.clone().into();
                if tx.hash() == curr_hash {
                    return true
                } else {
                    missing_hashes.push(curr_hash);
                }
            }
            false
        });

        // this means we have been sent transactions that we did not request
        if !unmatched_transactions.is_empty() {
            return Err(PooledTransactionsError::UnmatchedTransactions)
        }

        let matched_hashes = matched_transactions.iter().map(|tx| tx.hash()).collect::<Vec<H256>>();

        Ok((matched_hashes, missing_hashes))
    }
}

impl From<Vec<TransactionSigned>> for PooledTransactions {
    fn from(txs: Vec<TransactionSigned>) -> Self {
        PooledTransactions(txs)
    }
}

impl From<PooledTransactions> for Vec<TransactionSigned> {
    fn from(txs: PooledTransactions) -> Self {
        txs.0
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{message::RequestPair, GetPooledTransactions, PooledTransactions};
    use hex_literal::hex;
    use reth_primitives::{Signature, Transaction, TransactionKind, TransactionSigned, U256};
    use reth_rlp::{Decodable, Encodable};

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn encode_get_pooled_transactions() {
        let expected = hex!("f847820457f842a000000000000000000000000000000000000000000000000000000000deadc0dea000000000000000000000000000000000000000000000000000000000feedbeef");
        let mut data = vec![];
        let request = RequestPair::<GetPooledTransactions> {
            request_id: 1111,
            message: GetPooledTransactions(vec![
                hex!("00000000000000000000000000000000000000000000000000000000deadc0de").into(),
                hex!("00000000000000000000000000000000000000000000000000000000feedbeef").into(),
            ]),
        };
        request.encode(&mut data);
        assert_eq!(data, expected);
    }

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn decode_get_pooled_transactions() {
        let data = hex!("f847820457f842a000000000000000000000000000000000000000000000000000000000deadc0dea000000000000000000000000000000000000000000000000000000000feedbeef");
        let request = RequestPair::<GetPooledTransactions>::decode(&mut &data[..]).unwrap();
        assert_eq!(
            request,
            RequestPair::<GetPooledTransactions> {
                request_id: 1111,
                message: GetPooledTransactions(vec![
                    hex!("00000000000000000000000000000000000000000000000000000000deadc0de").into(),
                    hex!("00000000000000000000000000000000000000000000000000000000feedbeef").into(),
                ])
            }
        );
    }

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn encode_pooled_transactions() {
        let expected = hex!("f8d7820457f8d2f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb");
        let mut data = vec![];
        let request = RequestPair::<PooledTransactions> {
            request_id: 1111,
            message: vec![
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(1),
                        nonce: 0x8u64.into(),
                        gas_price: 0x4a817c808u64.into(),
                        gas_limit: 0x2e248u64.into(),
                        to: TransactionKind::Call(
                            hex!("3535353535353535353535353535353535353535").into(),
                        ),
                        value: 0x200u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(1),
                        nonce: 0x09u64.into(),
                        gas_price: 0x4a817c809u64.into(),
                        gas_limit: 0x33450u64.into(),
                        to: TransactionKind::Call(
                            hex!("3535353535353535353535353535353535353535").into(),
                        ),
                        value: 0x2d9u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                        )
                        .unwrap(),
                    },
                ),
            ]
            .into(),
        };
        request.encode(&mut data);
        assert_eq!(data, expected);
    }

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn decode_pooled_transactions() {
        let data = hex!("f8d7820457f8d2f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb");
        let expected = RequestPair::<PooledTransactions> {
            request_id: 1111,
            message: vec![
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(1),
                        nonce: 0x8u64.into(),
                        gas_price: 0x4a817c808u64.into(),
                        gas_limit: 0x2e248u64.into(),
                        to: TransactionKind::Call(
                            hex!("3535353535353535353535353535353535353535").into(),
                        ),
                        value: 0x200u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(1),
                        nonce: 0x09u64.into(),
                        gas_price: 0x4a817c809u64.into(),
                        gas_limit: 0x33450u64.into(),
                        to: TransactionKind::Call(
                            hex!("3535353535353535353535353535353535353535").into(),
                        ),
                        value: 0x2d9u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                        )
                        .unwrap(),
                    },
                ),
            ]
            .into(),
        };

        let request = RequestPair::<PooledTransactions>::decode(&mut &data[..]).unwrap();
        assert_eq!(request, expected);
    }

    #[test]
    fn decode_pooled_transactions_network() {
        let data = hex!("f9022980f90225f8650f84832156008287fb94cf7f9e66af820a19257a2108375b180b0ec491678204d2802ca035b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981a0612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860b87502f872041a8459682f008459682f0d8252089461815774383099e24810ab832a5b2a5425c154d58829a2241af62c000080c001a059e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafda0016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469f86b0384773594008398968094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba0ce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071a03ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88f86b01843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac3960468702769bb01b2a00802ba0e24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0aa05406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631daf86b02843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba00eb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5aea03a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18");
        let decoded_transactions =
            RequestPair::<PooledTransactions>::decode(&mut &data[..]).unwrap();

        let expected_transactions = RequestPair::<PooledTransactions> {
            request_id: 0,
            message: vec![
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 15u64.into(),
                        gas_price: 2200000000u64.into(),
                        gas_limit: 34811u64.into(),
                        to: TransactionKind::Call(
                            hex!("cf7f9e66af820a19257a2108375b180b0ec49167").into(),
                        ),
                        value: 1234u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: true,
                        r: U256::from_str(
                            "35b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Eip1559 {
                        chain_id: 4,
                        nonce: 26u64.into(),
                        max_priority_fee_per_gas: 1500000000u64.into(),
                        max_fee_per_gas: 1500000013u64.into(),
                        gas_limit: 21000u64.into(),
                        to: TransactionKind::Call(
                            hex!("61815774383099e24810ab832a5b2a5425c154d5").into(),
                        ),
                        value: 3000000000000000000u64.into(),
                        input: Default::default(),
                        access_list: Default::default(),
                    },
                    Signature {
                        odd_y_parity: true,
                        r: U256::from_str(
                            "59e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafd",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 3u64.into(),
                        gas_price: 2000000000u64.into(),
                        gas_limit: 10000000u64.into(),
                        to: TransactionKind::Call(
                            hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into(),
                        ),
                        value: 1000000000000000u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "ce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "3ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 1u64.into(),
                        gas_price: 1000000000u64.into(),
                        gas_limit: 100000u64.into(),
                        to: TransactionKind::Call(
                            hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into(),
                        ),
                        value: 693361000000000u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "e24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0a",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "5406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631da",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 2u64.into(),
                        gas_price: 1000000000u64.into(),
                        gas_limit: 100000u64.into(),
                        to: TransactionKind::Call(
                            hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into(),
                        ),
                        value: 1000000000000000u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "eb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5ae",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "3a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18",
                        )
                        .unwrap(),
                    },
                ),
            ]
            .into(),
        };

        // checking tx by tx for easier debugging if there are any regressions
        for (expected, decoded) in
            decoded_transactions.message.0.iter().zip(expected_transactions.message.0.iter())
        {
            assert_eq!(expected, decoded);
        }

        assert_eq!(decoded_transactions, expected_transactions);
    }

    #[test]
    fn encode_pooled_transactions_network() {
        let expected = hex!("f9022980f90225f8650f84832156008287fb94cf7f9e66af820a19257a2108375b180b0ec491678204d2802ca035b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981a0612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860b87502f872041a8459682f008459682f0d8252089461815774383099e24810ab832a5b2a5425c154d58829a2241af62c000080c001a059e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafda0016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469f86b0384773594008398968094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba0ce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071a03ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88f86b01843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac3960468702769bb01b2a00802ba0e24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0aa05406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631daf86b02843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba00eb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5aea03a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18");

        let transactions = RequestPair::<PooledTransactions> {
            request_id: 0,
            message: vec![
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 15u64.into(),
                        gas_price: 2200000000u64.into(),
                        gas_limit: 34811u64.into(),
                        to: TransactionKind::Call(
                            hex!("cf7f9e66af820a19257a2108375b180b0ec49167").into(),
                        ),
                        value: 1234u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: true,
                        r: U256::from_str(
                            "35b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Eip1559 {
                        chain_id: 4,
                        nonce: 26u64.into(),
                        max_priority_fee_per_gas: 1500000000u64.into(),
                        max_fee_per_gas: 1500000013u64.into(),
                        gas_limit: 21000u64.into(),
                        to: TransactionKind::Call(
                            hex!("61815774383099e24810ab832a5b2a5425c154d5").into(),
                        ),
                        value: 3000000000000000000u64.into(),
                        input: Default::default(),
                        access_list: Default::default(),
                    },
                    Signature {
                        odd_y_parity: true,
                        r: U256::from_str(
                            "59e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafd",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 3u64.into(),
                        gas_price: 2000000000u64.into(),
                        gas_limit: 10000000u64.into(),
                        to: TransactionKind::Call(
                            hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into(),
                        ),
                        value: 1000000000000000u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "ce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "3ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 1u64.into(),
                        gas_price: 1000000000u64.into(),
                        gas_limit: 100000u64.into(),
                        to: TransactionKind::Call(
                            hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into(),
                        ),
                        value: 693361000000000u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "e24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0a",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "5406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631da",
                        )
                        .unwrap(),
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    Transaction::Legacy {
                        chain_id: Some(4),
                        nonce: 2u64.into(),
                        gas_price: 1000000000u64.into(),
                        gas_limit: 100000u64.into(),
                        to: TransactionKind::Call(
                            hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into(),
                        ),
                        value: 1000000000000000u64.into(),
                        input: Default::default(),
                    },
                    Signature {
                        odd_y_parity: false,
                        r: U256::from_str(
                            "eb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5ae",
                        )
                        .unwrap(),
                        s: U256::from_str(
                            "3a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18",
                        )
                        .unwrap(),
                    },
                ),
            ]
            .into(),
        };

        let mut encoded = vec![];
        transactions.encode(&mut encoded);
        assert_eq!(encoded.len(), transactions.length());
        let encoded_str = hex::encode(encoded);
        let expected_str = hex::encode(expected);
        assert_eq!(encoded_str.len(), expected_str.len());
        assert_eq!(encoded_str, expected_str);
    }
}