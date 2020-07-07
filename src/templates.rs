/// Tx Request Template 
pub static BASIC_TRANSACTION_SEND_REQUEST : &'static str = r#"
{
  "requests": [
    {
      "address": "",
      "value": 5000000,
      "assets": [
        {
          "tokenId": "",
          "amount": 1
        }
      ],
      "registers": {
      }
    }
  ],
  "fee": 1000000,
  "inputsRaw": [],
  "dataInputsRaw": []
}"#;
