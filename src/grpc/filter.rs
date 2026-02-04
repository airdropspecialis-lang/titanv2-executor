use std::collections::HashMap;
use yellowstone_grpc_proto::geyser::SubscribeRequestFilterTransactions;

/// Raydium AMM v4 program (mainnet)
pub const RAYDIUM_AMM_V4_PROGRAM: &str =
    "675kPX9MHTjS2zt1qfr1NYHuHdiXESLiG1e66f4Hmcfs";

/// Builds a strict Yellowstone transaction filter that:
/// - excludes votes
/// - excludes failed transactions
/// - only streams transactions touching Raydium AMM v4
pub fn raydium_transaction_filters() -> HashMap<String, SubscribeRequestFilterTransactions> {
    let mut filters = HashMap::with_capacity(1);

    filters.insert(
        "raydium_amm_v4".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            signature: None,
            account_include: vec![RAYDIUM_AMM_V4_PROGRAM.to_string()],
            account_exclude: Vec::new(),
            account_required: Vec::new(),
        },
    );

    filters
}
