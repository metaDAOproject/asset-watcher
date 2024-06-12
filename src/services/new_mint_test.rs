#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::Connection;

    #[test]
    fn test_handle_mint_tx() {
        let database_url = "postgres://user:password@localhost/test";
        let mut connection =
            PgConnection::establish(&database_url).expect("Error connecting to the database");

        let payload = Payload {
            block_time: 1717700919,
            slot: 303970796,
            recent_blockhash: "B8LmpfZy9cAYRDuUcH2QGS2LLeTdvdjQFWQZy6eHoqR9".to_string(),
            compute_units_consumed: "BIGINT:90475".to_string(),
            fee: "BIGINT:5100".to_string(),
            signatures: vec!["3igJ9nsRaVjQ9ZjYuRHNJZBniBrTxYbdng8aXZ4DNMKgFjYdiypvtbgHsspo1kynG6C9nTEzNh5zJTSep8svVwnJ".to_string()],
            version: "legacy".to_string(),
            log_messages: vec!["Program ComputeBudget111111111111111111111111111111 invoke [1]".to_string()],
            accounts: vec![
                Account {
                    name: "authority".to_string(),
                    pubkey: "HwBL75xHHKcXSMNcctq3UqWaEJPDWVQz6NazZJNjWaQc".to_string(),
                    is_signer: true,
                    is_writeable: true,
                    pre_balance: Some("BIGINT:13114504583".to_string()),
                    post_balance: Some("BIGINT:13110420923".to_string()),
                    pre_token_balance: None,
                    post_token_balance: None,
                },
                Account {
                    name: "userConditionalOnFinalizeTokenAccount".to_string(),
                    pubkey: "6etKn4C4M8HGXECzTvfhAWzzLMfMmLX5Thcy7vm3Enjg".to_string(),
                    is_signer: false,
                    is_writeable: true,
                    pre_balance: Some("BIGINT:0".to_string()),
                    post_balance: Some("BIGINT:2039280".to_string()),
                    pre_token_balance: None,
                    post_token_balance: Some(TokenBalance {
                        mint: "Aq89VXVASGscd7fmQmoxooD41C4CDbKxWR7wAmmybczz".to_string(),
                        owner: "HwBL75xHHKcXSMNcctq3UqWaEJPDWVQz6NazZJNjWaQc".to_string(),
                        amount: "BIGINT:100000000000".to_string(),
                        decimals: 9,
                    }),
                },
                Account {
                    name: "userConditionalOnRevertTokenAccount".to_string(),
                    pubkey: "9mPrCP47iHhddGuPmAkcy46ywaYrdDbMi86u5nzwrU29".to_string(),
                    is_signer: false,
                    is_writeable: true,
                    pre_balance: Some("BIGINT:0".to_string()),
                    post_balance: Some("BIGINT:2039280".to_string()),
                    pre_token_balance: None,
                    post_token_balance: Some(TokenBalance {
                        mint: "BidfawNV22PCPvANvtP5wwYPBMMSn3RY8ehpBF3myF2L".to_string(),
                        owner: "HwBL75xHHKcXSMNcctq3UqWaEJPDWVQz6NazZJNjWaQc".to_string(),
                        amount: "BIGINT:100000000000".to_string(),
                        decimals: 9,
                    }),
                },
            ],
            instructions: vec![],
        };

        let result = handle_mint_tx(&mut connection, payload);
        assert!(result.is_ok());
    }
}
