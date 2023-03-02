use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use reqwest::Url;

pub const MAINNET_EXPLORER_API_URL: &str = "https://api.ergoplatform.com/";
pub const TESTNET_EXPLORER_API_URL: &str = "https://api-testnet.ergoplatform.com/";

pub const MAINNET_EXPLORER_URL: &str = "https://explorer.ergoplatform.com/";
pub const TESTNET_EXPLORER_URL: &str = "https://testnet.ergoplatform.com/";

pub fn default_explorer_api_url(network_prefix: NetworkPrefix) -> Url {
    let url_str = match network_prefix {
        NetworkPrefix::Mainnet => MAINNET_EXPLORER_API_URL,
        NetworkPrefix::Testnet => TESTNET_EXPLORER_API_URL,
    };
    Url::parse(url_str).unwrap()
}

pub fn default_explorer_url(network_prefix: NetworkPrefix) -> Url {
    let url_str = match network_prefix {
        NetworkPrefix::Mainnet => MAINNET_EXPLORER_URL,
        NetworkPrefix::Testnet => TESTNET_EXPLORER_URL,
    };
    Url::parse(url_str).unwrap()
}
