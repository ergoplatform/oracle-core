use futures::future::BoxFuture;

use super::assets_exchange_rate::AssetsExchangeRate;
use super::assets_exchange_rate::AssetsExchangeRateSource;
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::bitpanda::BitPanda;
use super::coincap::CoinCap;
use super::erg_xau::KgAu;
use super::DataPointSourceError;

pub struct BitPandaViaCoinCap;

impl AssetsExchangeRateSource<KgAu, NanoErg> for BitPandaViaCoinCap {
    fn get_rate(
        &self,
    ) -> BoxFuture<Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError>> {
        Box::pin(get_kgau_nanoerg())
    }
}

async fn get_kgau_nanoerg() -> Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError> {
    let kgau_usd_rate: AssetsExchangeRate<KgAu, Usd> = BitPanda {}.get_rate().await?;
    let usd_nanoerg_rate: AssetsExchangeRate<Usd, NanoErg> = CoinCap {}.get_rate().await?;
    let rate = kgau_usd_rate.rate * usd_nanoerg_rate.rate;
    Ok(AssetsExchangeRate {
        per1: KgAu {},
        get: NanoErg {},
        rate,
    })
}
