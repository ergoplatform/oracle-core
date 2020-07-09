/// Tx Request Template
pub static BASIC_TRANSACTION_SEND_REQUEST: &'static str = r#"
{
  "requests": [
    {
      "address": "",
      "value": 1000000,
      "assets": [],
      "registers": {}
    }
  ],
  "fee": 1000000,
  "inputsRaw": [],
  "dataInputsRaw": []
}"#;
