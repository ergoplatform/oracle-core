//! Extra price sources read directly from exchanges, plus the LiveCoinWatch and
//! CoinMarketCap aggregators, so a datapoint no longer depends on one or two
//! aggregator APIs (CoinGecko answers 403/429 to many hosts, CoinCap v2 is gone).
//!
//! Exchange prices are USDT pairs and are taken as USD. Gold is read from PAXG
//! (1 token = 1 troy ounce, redeemable), which tracks spot within a few tenths
//! of a percent.

use std::pin::Pin;
use std::time::Duration;

use futures::Future;
use json::JsonValue;

use super::assets_exchange_rate::{AssetsExchangeRate, NanoErg, Usd};
use super::erg_xau::KgAu;
use super::DataPointSourceError;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
// some of these APIs sit behind CDNs that reject reqwest's default (empty) user agent
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

#[allow(clippy::type_complexity)]
pub fn nanoerg_usd_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError>>>>,
> {
    vec![
        Box::pin(get_usd_nanoerg_kucoin()),
        Box::pin(get_usd_nanoerg_mexc()),
        Box::pin(get_usd_nanoerg_gate()),
        Box::pin(get_usd_nanoerg_livecoinwatch()),
        Box::pin(get_usd_nanoerg_coinmarketcap()),
    ]
}

async fn get_usd_nanoerg_kucoin() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError>
{
    let url = "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=ERG-USDT";
    let json = get_json(client()?.get(url)).await?;
    usd_nanoerg(&json, &["data", "price"])
}

async fn get_usd_nanoerg_mexc() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let url = "https://api.mexc.com/api/v3/ticker/price?symbol=ERGUSDT";
    let json = get_json(client()?.get(url)).await?;
    usd_nanoerg(&json, &["price"])
}

async fn get_usd_nanoerg_gate() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let url = "https://api.gateio.ws/api/v4/spot/tickers?currency_pair=ERG_USDT";
    let json = get_json(client()?.get(url)).await?;
    usd_nanoerg(&json[0], &["last"])
}

// Needs a free API key in the LCW_API_KEY env var; without it this source is skipped.
async fn get_usd_nanoerg_livecoinwatch(
) -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let api_key = std::env::var("LCW_API_KEY").map_err(|_| missing_field("LCW_API_KEY env var"))?;
    let req = client()?
        .post("https://api.livecoinwatch.com/coins/single")
        .header("x-api-key", api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(r#"{"currency":"USD","code":"ERG","meta":false}"#);
    let json = get_json(req).await?;
    usd_nanoerg(&json, &["rate"])
}

// Keyless public API (rate limited on rapid calls; fine at datapoint cadence). 1762 = Ergo.
async fn get_usd_nanoerg_coinmarketcap(
) -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let url = "https://pro-api.coinmarketcap.com/public-api/v2/simple/price?id=1762&convert=USD";
    let json = get_json(client()?.get(url)).await?;
    usd_nanoerg(&json["data"][0]["quotes"][0], &["price"])
}

/// Gold (USD per kg) from PAXG markets, for the ERG/XAU pool.
#[allow(clippy::type_complexity)]
pub fn kgau_usd_sources(
) -> Vec<Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError>>>>>
{
    vec![
        Box::pin(get_kgau_usd_kraken_paxg()),
        Box::pin(get_kgau_usd_kucoin_paxg()),
    ]
}

async fn get_kgau_usd_kraken_paxg() -> Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError> {
    let url = "https://api.kraken.com/0/public/Ticker?pair=PAXGUSD";
    let json = get_json(client()?.get(url)).await?;
    // "c" = last trade [price, lot volume]
    kgau_usd(&json["result"]["PAXGUSD"]["c"][0], "result.PAXGUSD.c[0]")
}

async fn get_kgau_usd_kucoin_paxg() -> Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError> {
    let url = "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=PAXG-USDT";
    let json = get_json(client()?.get(url)).await?;
    kgau_usd(&json["data"]["price"], "data.price")
}

fn client() -> Result<reqwest::Client, DataPointSourceError> {
    Ok(reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?)
}

async fn get_json(req: reqwest::RequestBuilder) -> Result<JsonValue, DataPointSourceError> {
    let resp = req.send().await?.error_for_status()?;
    Ok(json::parse(&resp.text().await?)?)
}

/// Reads the ERG price in USD at `path` (a number or a numeric string) and converts
/// it to nanoErgs per 1 USD.
fn usd_nanoerg(
    json: &JsonValue,
    path: &[&str],
) -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let field = path.join(".");
    let value = path.iter().fold(json, |j, key| &j[*key]);
    let usd_per_erg =
        positive_f64(value).ok_or_else(|| DataPointSourceError::JsonMissingField {
            field: format!("{field} as positive f64"),
            json: json.dump(),
        })?;
    Ok(AssetsExchangeRate {
        per1: Usd {},
        get: NanoErg {},
        rate: NanoErg::from_erg(1.0 / usd_per_erg),
    })
}

/// Reads the USD price of one troy ounce of gold and converts it to USD per kg.
fn kgau_usd(
    value: &JsonValue,
    field: &str,
) -> Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError> {
    let usd_per_oz = positive_f64(value).ok_or_else(|| DataPointSourceError::JsonMissingField {
        field: format!("{field} as positive f64"),
        json: value.dump(),
    })?;
    Ok(AssetsExchangeRate {
        per1: KgAu {},
        get: Usd {},
        rate: KgAu::from_troy_ounce(usd_per_oz),
    })
}

fn positive_f64(value: &JsonValue) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
        .filter(|p| p.is_finite() && *p > 0.0)
}

fn missing_field(field: &str) -> DataPointSourceError {
    DataPointSourceError::JsonMissingField {
        field: field.to_string(),
        json: String::new(),
    }
}
