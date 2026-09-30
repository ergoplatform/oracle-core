//! Extra price sources read directly from exchanges, plus the LiveCoinWatch and
//! CoinMarketCap aggregators, so a datapoint no longer depends on one or two
//! aggregator APIs (CoinGecko answers 403/429 to many hosts, CoinCap v2 is gone).
//!
//! Exchange prices are USDT pairs and are taken as USD. Gold comes from COMEX
//! futures and from PAXG (1 token = 1 troy ounce, redeemable), which tracks spot
//! within a few tenths of a percent.
//!
//! Where an API returns the time of its quote, a quote older than the limit for
//! that source is rejected instead of counted.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use json::JsonValue;

use super::aggregator::{fetch_aggregated, source, RateResult, Source};
use super::assets_exchange_rate::{AssetsExchangeRate, NanoErg, Usd};
use super::erg_xau::KgAu;
use super::DataPointSourceError;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
// some of these APIs sit behind CDNs that reject reqwest's default (empty) user agent
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
/// Limit for KuCoin and CoinMarketCap quotes.
const MAX_QUOTE_AGE: Duration = Duration::from_secs(5 * 60);
/// Yahoo's COMEX quote runs ~10 min behind, so an older quote means the market
/// is closed (weekend, holiday, the daily CME break).
const MAX_COMEX_QUOTE_AGE: Duration = Duration::from_secs(30 * 60);

pub fn nanoerg_usd_sources() -> Vec<Source<Usd, NanoErg>> {
    let mut sources = vec![
        source("kucoin", get_usd_nanoerg_kucoin()),
        source("mexc", get_usd_nanoerg_mexc()),
        source("gate", get_usd_nanoerg_gate()),
        source("coinmarketcap", get_usd_nanoerg_coinmarketcap()),
    ];
    // needs a free API key; without one it is left out rather than failing every round
    if let Some(api_key) = std::env::var("LCW_API_KEY").ok().filter(|k| !k.is_empty()) {
        sources.push(source(
            "livecoinwatch",
            get_usd_nanoerg_livecoinwatch(api_key),
        ));
    }
    sources
}

async fn get_usd_nanoerg_kucoin() -> RateResult<Usd, NanoErg> {
    let url = "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=ERG-USDT";
    let json = get_json(client()?.get(url)).await?;
    Ok(usd_nanoerg(kucoin_price(&json, now_ms())?))
}

async fn get_usd_nanoerg_mexc() -> RateResult<Usd, NanoErg> {
    let url = "https://api.mexc.com/api/v3/ticker/price?symbol=ERGUSDT";
    let json = get_json(client()?.get(url)).await?;
    Ok(usd_nanoerg(price(&json, &json["price"], "price")?))
}

async fn get_usd_nanoerg_gate() -> RateResult<Usd, NanoErg> {
    let url = "https://api.gateio.ws/api/v4/spot/tickers?currency_pair=ERG_USDT";
    let json = get_json(client()?.get(url)).await?;
    Ok(usd_nanoerg(price(&json, &json[0]["last"], "[0].last")?))
}

async fn get_usd_nanoerg_livecoinwatch(api_key: String) -> RateResult<Usd, NanoErg> {
    let req = client()?
        .post("https://api.livecoinwatch.com/coins/single")
        .header("x-api-key", api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(r#"{"currency":"USD","code":"ERG","meta":false}"#);
    let json = get_json(req).await?;
    Ok(usd_nanoerg(price(&json, &json["rate"], "rate")?))
}

// Keyless public API (rate limited on rapid calls; fine at datapoint cadence). 1762 = Ergo.
async fn get_usd_nanoerg_coinmarketcap() -> RateResult<Usd, NanoErg> {
    let url = "https://pro-api.coinmarketcap.com/public-api/v2/simple/price?id=1762&convert=USD&include_last_updated=true";
    let json = get_json(client()?.get(url)).await?;
    Ok(usd_nanoerg(coinmarketcap_price(&json, now_ms())?))
}

/// Gold (USD per kg) for the ERG/XAU pool, beside Bitpanda. Kraken and KuCoin
/// both quote PAXG, so they make one vote (their mean) rather than two, and one
/// proxy asset cannot outvote Bitpanda and COMEX.
pub fn kgau_usd_sources() -> Vec<Source<KgAu, Usd>> {
    vec![
        source("comex", get_kgau_usd_comex()),
        source("paxg", get_kgau_usd_paxg()),
    ]
}

/// COMEX gold futures (GC=F, front month) from Yahoo Finance's chart endpoint.
/// The endpoint is undocumented and the quote is delayed, so this is an expendable
/// vote: a quote older than MAX_COMEX_QUOTE_AGE is rejected, and a Friday close
/// never counts as a weekend gold price. Yahoo answered 429 to no user agent, to
/// curl's and to a full browser one in testing, but not to a bare "Mozilla/5.0".
async fn get_kgau_usd_comex() -> RateResult<KgAu, Usd> {
    let url = "https://query1.finance.yahoo.com/v8/finance/chart/GC=F?interval=1d&range=1d";
    let json = get_json(client_with_user_agent("Mozilla/5.0")?.get(url)).await?;
    Ok(kgau_usd(yahoo_price(&json, now_ms())?))
}

async fn get_kgau_usd_paxg() -> RateResult<KgAu, Usd> {
    let venues = vec![
        source("kraken", get_kgau_usd_kraken_paxg()),
        source("kucoin", get_kgau_usd_kucoin_paxg()),
    ];
    fetch_aggregated("PAXG", venues, 1).await
}

async fn get_kgau_usd_kraken_paxg() -> RateResult<KgAu, Usd> {
    let url = "https://api.kraken.com/0/public/Ticker?pair=PAXGUSD";
    let json = get_json(client()?.get(url)).await?;
    // "c" = last trade [price, lot volume]
    let c = &json["result"]["PAXGUSD"]["c"][0];
    Ok(kgau_usd(price(&json, c, "result.PAXGUSD.c[0]")?))
}

async fn get_kgau_usd_kucoin_paxg() -> RateResult<KgAu, Usd> {
    let url = "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=PAXG-USDT";
    let json = get_json(client()?.get(url)).await?;
    Ok(kgau_usd(kucoin_price(&json, now_ms())?))
}

fn kucoin_price(json: &JsonValue, now_ms: i64) -> Result<f64, DataPointSourceError> {
    let time_ms = json["data"]["time"]
        .as_i64()
        .ok_or_else(|| missing_field(json, "data.time as integer"))?;
    check_fresh(time_ms, now_ms, MAX_QUOTE_AGE, "data.time")?;
    price(json, &json["data"]["price"], "data.price")
}

fn coinmarketcap_price(json: &JsonValue, now_ms: i64) -> Result<f64, DataPointSourceError> {
    let quote = &json["data"][0]["quotes"][0];
    let field = "data[0].quotes[0].last_updated";
    let updated = quote["last_updated"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .ok_or_else(|| missing_field(json, &format!("{field} as RFC 3339 time")))?;
    check_fresh(updated.timestamp_millis(), now_ms, MAX_QUOTE_AGE, field)?;
    price(json, &quote["price"], "data[0].quotes[0].price")
}

fn yahoo_price(json: &JsonValue, now_ms: i64) -> Result<f64, DataPointSourceError> {
    let meta = &json["chart"]["result"][0]["meta"];
    let field = "chart.result[0].meta.regularMarketTime";
    let time_s = meta["regularMarketTime"]
        .as_i64()
        .ok_or_else(|| missing_field(json, &format!("{field} as integer")))?;
    check_fresh(time_s * 1000, now_ms, MAX_COMEX_QUOTE_AGE, field)?;
    price(
        json,
        &meta["regularMarketPrice"],
        "chart.result[0].meta.regularMarketPrice",
    )
}

fn client() -> Result<reqwest::Client, DataPointSourceError> {
    client_with_user_agent(USER_AGENT)
}

fn client_with_user_agent(user_agent: &str) -> Result<reqwest::Client, DataPointSourceError> {
    Ok(reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(user_agent)
        .build()?)
}

async fn get_json(req: reqwest::RequestBuilder) -> Result<JsonValue, DataPointSourceError> {
    let resp = req.send().await?.error_for_status()?;
    Ok(json::parse(&resp.text().await?)?)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Fails if a quote from `quote_ms` (unix ms) is older than `max_age` at `now_ms`.
fn check_fresh(
    quote_ms: i64,
    now_ms: i64,
    max_age: Duration,
    field: &str,
) -> Result<(), DataPointSourceError> {
    let age_ms = now_ms - quote_ms;
    if age_ms > max_age.as_millis() as i64 {
        Err(DataPointSourceError::StaleQuote {
            field: field.to_string(),
            age_secs: age_ms / 1000,
            max_age_secs: max_age.as_secs(),
        })
    } else {
        Ok(())
    }
}

/// Reads a positive price (a number or a numeric string) from `value`, a node of `json`.
fn price(json: &JsonValue, value: &JsonValue, field: &str) -> Result<f64, DataPointSourceError> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
        .filter(|p| p.is_finite() && *p > 0.0)
        .ok_or_else(|| missing_field(json, &format!("{field} as positive f64")))
}

/// nanoErgs per 1 USD from the USD price of 1 ERG
fn usd_nanoerg(usd_per_erg: f64) -> AssetsExchangeRate<Usd, NanoErg> {
    AssetsExchangeRate {
        per1: Usd {},
        get: NanoErg {},
        rate: NanoErg::from_erg(1.0 / usd_per_erg),
    }
}

/// USD per kg of gold from the USD price of 1 troy ounce
fn kgau_usd(usd_per_oz: f64) -> AssetsExchangeRate<KgAu, Usd> {
    AssetsExchangeRate {
        per1: KgAu {},
        get: Usd {},
        rate: KgAu::from_troy_ounce(usd_per_oz),
    }
}

fn missing_field(json: &JsonValue, field: &str) -> DataPointSourceError {
    DataPointSourceError::JsonMissingField {
        field: field.to_string(),
        json: json.dump(),
    }
}
