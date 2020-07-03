/// This file holds various functions related to encoding/serialization of values that are relevant
/// to the oracle core.
use sigma_tree::ast::{CollPrim, Constant, ConstantVal};
use std::str;


/// Serialize a `i64` value into a hex-encoded string to be used inside of a register for a box
pub fn serialize_integer(i: i64) -> String {
    let c = serde_json::to_string_pretty(&Constant::long(i)).unwrap();
    (c[1..(c.len() -1)]).to_string()

}

/// Serialize a `String` value into a hex-encoded string to be used inside of a register for a box
pub fn serialize_string(s: &String) -> String {
    let a = s.clone().into_bytes();
    let b = a.iter().map(|c| c.clone() as i8).collect();
    let c = serde_json::to_string(&Constant::byte_array(b)).unwrap();
    (c[1..(c.len() -1)]).to_string()
}


/// Deserialize a hex-encoded `i64` inside of a `Constant` acquired from a register of a box
pub fn deserialize_integer(c: &Constant) -> Option<i64> {
    match &c.v {
        ConstantVal::Long(i) => return Some(i.clone()),
        _ => return None
    };
}

/// Deserialize a hex-encoded string inside of a `Constant` acquired from a register of a box
pub fn deserialize_string(c: &Constant) -> Option<String> {
    let byte_array = match &c.v {
        ConstantVal::CollPrim(CollPrim::CollByte(ba)) => ba.iter().map(|x| x.clone() as u8).collect(),
        _ => vec![]
    };
    Some(str::from_utf8(&byte_array).ok()?.to_string())
}


/// Convert from Erg to nanoErg
pub fn erg_to_nanoerg(erg_amount: f64) -> u64 {
    (erg_amount * 1000000000 as f64) as u64
}

/// Convert from nanoErg to Erg
pub fn nanoerg_to_erg(nanoerg_amount: u64) -> f64 {
    (nanoerg_amount as f64) / (1000000000 as f64)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erg_conv_is_valid() {
        assert_eq!((1 as f64), nanoerg_to_erg(1000000000));
        assert_eq!((1.23 as f64), nanoerg_to_erg(1230000000));

        assert_eq!(1000000000, erg_to_nanoerg(1 as f64));
        assert_eq!(erg_to_nanoerg(3.64), 3640000000);
        assert_eq!(erg_to_nanoerg(0.64), 640000000);
        assert_eq!(erg_to_nanoerg(0.0064), 6400000);
        assert_eq!(erg_to_nanoerg(0.000000064), 64);
        assert_eq!(erg_to_nanoerg(0.000000001), 1);
    }
}