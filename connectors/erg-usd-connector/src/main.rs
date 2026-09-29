/// This Connector obtains the nanoErg per 1 USD rate and submits it
/// to an oracle core. It reads the `oracle-config.yaml` to find the port
/// of the oracle core (via Connector-Lib) and submits it to the POST API
/// server on the core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
///
/// The price is the median of several independent sources (aggregators and
/// exchange tickers) so one API going down or returning a bad price does not
/// stop or skew the datapoint. Exchange prices are USDT pairs, taken as USD.
use anyhow::{anyhow, Result};
use frontend_connector_lib::FrontendConnector;
use json::JsonValue;
use std::thread;
use std::time::Duration;

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// Some of these APIs sit behind CDNs that reject an empty user agent
static USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

/// Returns the price of 1 Erg in USD
type PriceSource = fn() -> Result<f64>;

static SOURCES: &[(&str, PriceSource)] = &[
    ("CoinGecko", coingecko),
    ("CoinPaprika", coinpaprika),
    ("CoinMarketCap", coinmarketcap),
    ("LiveCoinWatch", livecoinwatch),
    ("Bitpanda", bitpanda),
    ("KuCoin", kucoin),
    ("MEXC", mexc),
    ("Gate", gate),
];

/// Get the Erg/USD price from the nanoErgs per 1 USD datapoint price
pub fn generate_current_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * NANO_ERG_CONVERSION
}

/// Acquires the price of Ergs in USD from all sources in parallel, takes the
/// median of the ones that answered, converts it into nanoErgs per 1 USD, and
/// returns it.
fn get_nanoerg_usd_price() -> Result<u64> {
    let handles: Vec<_> = SOURCES
        .iter()
        .map(|(name, fetch)| (*name, thread::spawn(*fetch)))
        .collect();
    let mut prices = Vec::new();
    let mut errors = Vec::new();
    for (name, handle) in handles {
        match handle.join() {
            Ok(Ok(p)) if p.is_finite() && p > 0.0 => prices.push(p),
            Ok(Ok(p)) => errors.push(format!("{}: bad price {}", name, p)),
            Ok(Err(e)) => errors.push(format!("{}: {}", name, e)),
            Err(_) => errors.push(format!("{}: panicked", name)),
        }
    }
    if prices.is_empty() {
        return Err(anyhow!("No price from any source: {}", errors.join("; ")));
    }
    // Convert from price Erg/USD to nanoErgs per 1 USD
    let nanoerg_price = (1.0 / median(prices)) * NANO_ERG_CONVERSION;
    Ok(nanoerg_price as u64)
}

fn median(mut prices: Vec<f64>) -> f64 {
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // the two middle elements; the same element when the count is odd
    let n = prices.len();
    (prices[(n - 1) / 2] + prices[n / 2]) / 2.0
}

fn coingecko() -> Result<f64> {
    let json =
        get_json("https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD")?;
    price_at(&json["ergo"]["usd"])
}

fn coinpaprika() -> Result<f64> {
    let json = get_json("https://api.coinpaprika.com/v1/tickers/efyt-ergo")?;
    price_at(&json["quotes"]["USD"]["price"])
}

// Keyless public API (rate limited on rapid calls; fine at connector cadence). 1762 = Ergo.
fn coinmarketcap() -> Result<f64> {
    let json = get_json(
        "https://pro-api.coinmarketcap.com/public-api/v2/simple/price?id=1762&convert=USD",
    )?;
    price_at(&json["data"][0]["quotes"][0]["price"])
}

// Needs a free API key in the LCW_API_KEY env var; without it this source is skipped.
fn livecoinwatch() -> Result<f64> {
    let api_key = std::env::var("LCW_API_KEY").map_err(|_| anyhow!("LCW_API_KEY not set"))?;
    let resp = client()?
        .post("https://api.livecoinwatch.com/coins/single")
        .header("x-api-key", api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(r#"{"currency":"USD","code":"ERG","meta":false}"#)
        .send()?
        .error_for_status()?;
    let json = json::parse(&resp.text()?)?;
    price_at(&json["rate"])
}

fn bitpanda() -> Result<f64> {
    let json = get_json("https://api.bitpanda.com/v1/ticker")?;
    price_at(&json["ERG"]["USD"])
}

fn kucoin() -> Result<f64> {
    let json = get_json("https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=ERG-USDT")?;
    price_at(&json["data"]["price"])
}

fn mexc() -> Result<f64> {
    let json = get_json("https://api.mexc.com/api/v3/ticker/price?symbol=ERGUSDT")?;
    price_at(&json["price"])
}

fn gate() -> Result<f64> {
    let json = get_json("https://api.gateio.ws/api/v4/spot/tickers?currency_pair=ERG_USDT")?;
    price_at(&json[0]["last"])
}

fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?)
}

fn get_json(url: &str) -> Result<JsonValue> {
    let resp = client()?.get(url).send()?.error_for_status()?;
    Ok(json::parse(&resp.text()?)?)
}

/// Reads a price given either as a JSON number or as a numeric string
fn price_at(value: &JsonValue) -> Result<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
        .ok_or_else(|| anyhow!("Failed to parse price from json: {}", value.dump()))
}

fn main() {
    // Create the FrontendConnector
    let connector = FrontendConnector::new_basic_connector(
        "Erg-USD",
        get_nanoerg_usd_price,
        generate_current_price,
    );

    // Start the FrontendConnector
    connector.run();
}
