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

use std::time::Duration;

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
/// How far a quote time may be ahead of this host's clock.
const MAX_CLOCK_SKEW: Duration = Duration::from_secs(60);

pub fn nanoerg_usd_sources(livecoinwatch_api_key: Option<&str>) -> Vec<Source<Usd, NanoErg>> {
    let mut sources = vec![
        source("kucoin", get_usd_nanoerg_kucoin()),
        source("mexc", get_usd_nanoerg_mexc()),
        source("gate", get_usd_nanoerg_gate()),
        source("coinmarketcap", get_usd_nanoerg_coinmarketcap()),
    ];
    // needs a free API key (`livecoinwatch_api_key` in oracle_config.yaml); without
    // one it is left out rather than failing every round
    if let Some(api_key) = livecoinwatch_api_key.filter(|k| !k.is_empty()) {
        sources.push(source(
            "livecoinwatch",
            get_usd_nanoerg_livecoinwatch(api_key.to_string()),
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

// Both venues use REQUEST_TIMEOUT (10 s), so this vote ends inside the
// aggregator's 15 s per-source timeout.
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

#[cfg(not(test))]
async fn get_json(req: reqwest::RequestBuilder) -> Result<JsonValue, DataPointSourceError> {
    let resp = req.send().await?.error_for_status()?;
    Ok(json::parse(&resp.text().await?)?)
}

// tests read a recorded response instead of the network
#[cfg(test)]
async fn get_json(req: reqwest::RequestBuilder) -> Result<JsonValue, DataPointSourceError> {
    let url = req.build()?.url().to_string();
    Ok(json::parse(tests::fixture(&url))?)
}

#[cfg(not(test))]
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

// the time the fixtures were recorded
#[cfg(test)]
fn now_ms() -> i64 {
    tests::FIXTURE_NOW_MS
}

/// Fails if a quote from `quote_ms` (unix ms) is older than `max_age` at `now_ms`,
/// or more than MAX_CLOCK_SKEW ahead of it (a timestamp in another unit, or a host
/// clock that is off, would otherwise pass any age check).
fn check_fresh(
    quote_ms: i64,
    now_ms: i64,
    max_age: Duration,
    field: &str,
) -> Result<(), DataPointSourceError> {
    let age_ms = now_ms - quote_ms;
    if -age_ms > MAX_CLOCK_SKEW.as_millis() as i64 {
        Err(DataPointSourceError::FutureQuote {
            field: field.to_string(),
            ahead_secs: -age_ms / 1000,
        })
    } else if age_ms > max_age.as_millis() as i64 {
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

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use futures::future::ready;

    // Recorded responses (2026-09-30T10:24:28Z), with prices set to agree with the
    // other sources' test mocks (ERG ~1.669 USD, gold ~2056 USD/oz).
    pub const FIXTURE_NOW_MS: i64 = 1_790_763_868_000;

    const KUCOIN_ERG: &str = r#"{"code":"200000","data":{"time":1790763843281,"sequence":"1380393571","price":"1.6688","size":"9.9483","bestBid":"1.6685","bestBidSize":"29.5072","bestAsk":"1.6692","bestAskSize":"8.5412"}}"#;
    const MEXC: &str = r#"{"symbol":"ERGUSDT","price":"1.6702"}"#;
    const GATE: &str = r#"[{"currency_pair":"ERG_USDT","last":"1.6695","lowest_ask":"1.6707","lowest_size":"111.76","highest_bid":"1.6688","highest_size":"10.03","change_percentage":"5.51","base_volume":"87940.63","quote_volume":"146386.18","high_24h":"1.71","low_24h":"1.62"}]"#;
    const LIVECOINWATCH: &str = r#"{"rate":1.6711,"volume":301234,"cap":142876543,"liquidity":98765,"delta":{"hour":1.001,"day":1.05,"week":1.02,"month":0.97,"quarter":0.91,"year":0.8}}"#;
    const COINMARKETCAP: &str = r#"{"data":[{"id":1762,"name":"Ergo","symbol":"ERG","slug":"ergo","quotes":[{"symbol":"USD","price":1.66931,"last_updated":"2026-09-30T10:22:59.000Z"}]}],"status":{"timestamp":"2026-09-30T10:24:28.768Z","error_code":"0","error_message":"","elapsed":2,"credit_count":1}}"#;
    const YAHOO_GC: &str = r#"{"chart":{"result":[{"meta":{"currency":"USD","symbol":"GC=F","exchangeName":"CMX","fullExchangeName":"COMEX","instrumentType":"FUTURE","firstTradeDate":967608000,"regularMarketTime":1790763274,"hasPrePostMarketData":false,"gmtoffset":-14400,"timezone":"EDT","exchangeTimezoneName":"America/New_York","regularMarketPrice":2061.4,"chartPreviousClose":2052.3,"priceHint":2,"dataGranularity":"1d","range":"1d"},"timestamp":[1790763274],"indicators":{"quote":[{"close":[2061.4]}]}}],"error":null}}"#;
    const KRAKEN_PAXG: &str = r#"{"error":[],"result":{"PAXGUSD":{"a":["2055.150000","1","1.000"],"b":["2054.780000","1","1.000"],"c":["2054.870000","0.00100000"],"v":["80.47533909","380.47600788"],"p":["2054.494144","2051.965353"],"t":[2502,6319],"l":["2040.110000","2038.260000"],"h":["2060.670000","2060.670000"],"o":"2052.810000"}}}"#;
    const KUCOIN_PAXG: &str = r#"{"code":"200000","data":{"time":1790763824654,"sequence":"1835892252","price":"2056.12","size":"0.0007","bestBid":"2055.08","bestBidSize":"0.4556","bestAsk":"2056.09","bestAskSize":"0.2385"}}"#;

    pub fn fixture(url: &str) -> &'static str {
        match url {
            "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=ERG-USDT" => KUCOIN_ERG,
            "https://api.mexc.com/api/v3/ticker/price?symbol=ERGUSDT" => MEXC,
            "https://api.gateio.ws/api/v4/spot/tickers?currency_pair=ERG_USDT" => GATE,
            "https://api.livecoinwatch.com/coins/single" => LIVECOINWATCH,
            "https://pro-api.coinmarketcap.com/public-api/v2/simple/price?id=1762&convert=USD&include_last_updated=true" => COINMARKETCAP,
            "https://query1.finance.yahoo.com/v8/finance/chart/GC=F?interval=1d&range=1d" => YAHOO_GC,
            "https://api.kraken.com/0/public/Ticker?pair=PAXGUSD" => KRAKEN_PAXG,
            "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=PAXG-USDT" => KUCOIN_PAXG,
            _ => panic!("no fixture for {url}"),
        }
    }

    fn parse(s: &str) -> JsonValue {
        json::parse(s).unwrap()
    }

    /// `json` with the node at `path` replaced by `value`
    fn with(s: &str, path: &[&str], value: JsonValue) -> JsonValue {
        let mut json = parse(s);
        let (last, parents) = path.split_last().unwrap();
        let node = parents
            .iter()
            .fold(&mut json, |j, key| match key.parse::<usize>() {
                Ok(i) => &mut j[i],
                Err(_) => &mut j[*key],
            });
        node[*last] = value;
        json
    }

    #[test]
    fn test_parsers() {
        let now = FIXTURE_NOW_MS;
        assert_eq!(kucoin_price(&parse(KUCOIN_ERG), now).unwrap(), 1.6688);
        assert_eq!(kucoin_price(&parse(KUCOIN_PAXG), now).unwrap(), 2056.12);
        assert_eq!(
            coinmarketcap_price(&parse(COINMARKETCAP), now).unwrap(),
            1.66931
        );
        assert_eq!(yahoo_price(&parse(YAHOO_GC), now).unwrap(), 2061.4);
    }

    #[test]
    fn test_sources_on_fixtures() {
        use tokio_test::block_on;
        let usd_per_erg = |r: RateResult<Usd, NanoErg>| 1e9 / r.unwrap().rate;
        let usd_per_oz = |r: RateResult<KgAu, Usd>| r.unwrap().rate / KgAu::from_troy_ounce(1.0);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(close(
            usd_per_erg(block_on(get_usd_nanoerg_kucoin())),
            1.6688
        ));
        assert!(close(usd_per_erg(block_on(get_usd_nanoerg_mexc())), 1.6702));
        assert!(close(usd_per_erg(block_on(get_usd_nanoerg_gate())), 1.6695));
        let lcw = block_on(get_usd_nanoerg_livecoinwatch("key".into()));
        assert!(close(usd_per_erg(lcw), 1.6711));
        let cmc = block_on(get_usd_nanoerg_coinmarketcap());
        assert!(close(usd_per_erg(cmc), 1.66931));
        assert!(close(usd_per_oz(block_on(get_kgau_usd_comex())), 2061.4));
        assert!(close(
            usd_per_oz(block_on(get_kgau_usd_kraken_paxg())),
            2054.87
        ));
        assert!(close(
            usd_per_oz(block_on(get_kgau_usd_kucoin_paxg())),
            2056.12
        ));
        // the two PAXG venues make one vote, their mean
        let paxg = block_on(get_kgau_usd_paxg());
        assert!(close(usd_per_oz(paxg), (2054.87 + 2056.12) / 2.0));
    }

    #[test]
    fn test_stale_quotes_rejected() {
        let stale = |r: Result<f64, DataPointSourceError>| {
            matches!(r, Err(DataPointSourceError::StaleQuote { .. }))
        };
        let min = 60_000;

        // KuCoin: data.time 1790763843281, limit 5 min
        let kucoin = parse(KUCOIN_ERG);
        assert!(kucoin_price(&kucoin, 1_790_763_843_281 + 5 * min).is_ok());
        assert!(stale(kucoin_price(
            &kucoin,
            1_790_763_843_281 + 5 * min + 1000
        )));

        // CoinMarketCap: last_updated 10:22:59Z, limit 5 min
        let cmc = parse(COINMARKETCAP);
        assert!(coinmarketcap_price(&cmc, 1_790_763_779_000 + 5 * min).is_ok());
        assert!(stale(coinmarketcap_price(
            &cmc,
            1_790_763_779_000 + 5 * min + 1000
        )));

        // Yahoo: regularMarketTime 1790763274 (s), limit 30 min
        let yahoo = parse(YAHOO_GC);
        assert!(yahoo_price(&yahoo, 1_790_763_274_000 + 30 * min).is_ok());
        assert!(stale(yahoo_price(
            &yahoo,
            1_790_763_274_000 + 30 * min + 1000
        )));
    }

    #[test]
    fn test_future_quotes_rejected() {
        let future = |r: Result<f64, DataPointSourceError>| {
            matches!(r, Err(DataPointSourceError::FutureQuote { .. }))
        };
        let kucoin_ms = 1_790_763_843_281;
        let kucoin = parse(KUCOIN_ERG);
        // up to a minute of clock skew is fine
        assert!(kucoin_price(&kucoin, kucoin_ms - 60_000).is_ok());
        assert!(future(kucoin_price(&kucoin, kucoin_ms - 61_000)));
        // a time in another unit: nanoseconds from KuCoin, milliseconds from Yahoo
        let kucoin_ns = with(
            KUCOIN_ERG,
            &["data", "time"],
            1_790_763_843_281_000_000_i64.into(),
        );
        assert!(future(kucoin_price(&kucoin_ns, FIXTURE_NOW_MS)));
        let yahoo_ms = with(
            YAHOO_GC,
            &["chart", "result", "0", "meta", "regularMarketTime"],
            1_790_763_274_000_i64.into(),
        );
        assert!(future(yahoo_price(&yahoo_ms, FIXTURE_NOW_MS)));
    }

    #[test]
    fn test_missing_timestamp_rejected() {
        let missing = |r: Result<f64, DataPointSourceError>| {
            matches!(r, Err(DataPointSourceError::JsonMissingField { .. }))
        };
        let now = FIXTURE_NOW_MS;
        let kucoin = with(KUCOIN_ERG, &["data", "time"], JsonValue::Null);
        assert!(missing(kucoin_price(&kucoin, now)));
        // the response without include_last_updated=true
        let cmc = with(
            COINMARKETCAP,
            &["data", "0", "quotes", "0", "last_updated"],
            JsonValue::Null,
        );
        assert!(missing(coinmarketcap_price(&cmc, now)));
        let cmc = with(
            COINMARKETCAP,
            &["data", "0", "quotes", "0", "last_updated"],
            "yesterday".into(),
        );
        assert!(missing(coinmarketcap_price(&cmc, now)));
        let yahoo = with(
            YAHOO_GC,
            &["chart", "result", "0", "meta", "regularMarketTime"],
            JsonValue::Null,
        );
        assert!(missing(yahoo_price(&yahoo, now)));
    }

    #[test]
    fn test_bad_prices_rejected() {
        let now = FIXTURE_NOW_MS;
        for bad in [
            JsonValue::Null,
            "".into(),
            "abc".into(),
            "0".into(),
            "-1.5".into(),
            "NaN".into(),
            "inf".into(),
            0.into(),
            (-2.0).into(),
        ] {
            let kucoin = with(KUCOIN_ERG, &["data", "price"], bad.clone());
            assert!(
                matches!(
                    kucoin_price(&kucoin, now),
                    Err(DataPointSourceError::JsonMissingField { .. })
                ),
                "{bad} accepted"
            );
        }
    }

    fn gold(usd_per_oz: f64) -> RateResult<KgAu, Usd> {
        Ok(kgau_usd(usd_per_oz))
    }

    fn comex_at(now_ms: i64) -> RateResult<KgAu, Usd> {
        Ok(kgau_usd(yahoo_price(&parse(YAHOO_GC), now_ms)?))
    }

    fn gold_usd(sources: Vec<Source<KgAu, Usd>>) -> Result<f64, DataPointSourceError> {
        let rate = tokio_test::block_on(fetch_aggregated("gold/USD", sources, 2))?;
        Ok(rate.rate / KgAu::from_troy_ounce(1.0))
    }

    #[test]
    fn test_gold_weekday() {
        // all three votes count: the median is the middle one
        let sources = vec![
            source("bitpanda", ready(gold(2050.0))),
            source("comex", ready(comex_at(FIXTURE_NOW_MS))),
            source("paxg", ready(gold(2055.0))),
        ];
        assert!((gold_usd(sources).unwrap() - 2055.0).abs() < 1e-9);
    }

    #[test]
    fn test_gold_weekend() {
        // the fixture's COMEX quote read as Friday's close (21:00Z), on Saturday 15:00Z
        let yahoo = with(
            YAHOO_GC,
            &["chart", "result", "0", "meta", "regularMarketTime"],
            1_790_370_000.into(),
        );
        let saturday_ms = 1_790_434_800_000;
        let comex = || Ok(kgau_usd(yahoo_price(&yahoo, saturday_ms)?));
        assert!(matches!(
            comex(),
            Err(DataPointSourceError::StaleQuote { .. })
        ));

        // Bitpanda and PAXG live: gold is their midpoint, the Friday close does not count
        let sources = vec![
            source("bitpanda", ready(gold(2050.0))),
            source("comex", ready(comex())),
            source("paxg", ready(gold(2055.0))),
        ];
        assert!((gold_usd(sources).unwrap() - 2052.5).abs() < 1e-9);

        // PAXG down as well: one live vote is below the minimum of 2, so no gold price
        // rather than one source's (or a stale close's) price
        let sources = vec![
            source("bitpanda", ready(gold(2050.0))),
            source("comex", ready(comex())),
            source(
                "paxg",
                ready(Err::<AssetsExchangeRate<KgAu, Usd>, _>(
                    DataPointSourceError::NoDataPoints,
                )),
            ),
        ];
        assert!(matches!(
            gold_usd(sources),
            Err(DataPointSourceError::NotEnoughSources { got: 1, min: 2, .. })
        ));
    }
}
