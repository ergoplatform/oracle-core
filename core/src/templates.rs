/// Tx Request Template
pub static BASIC_TRANSACTION_SEND_REQUEST: &str = r#"
{
  "requests": [
    {
      "address": "",
      "value": 1000000,
      "assets": [],
      "registers": {}
    }
  ],
  "fee": 1,
  "inputsRaw": [],
  "dataInputsRaw": []
}"#;
