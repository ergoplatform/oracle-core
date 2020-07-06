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
        "R4": "",
        "R5": "",
        "R6": ""
      }
    }
  ],
  "fee": 1000000,
  "inputsRaw": [],
  "dataInputsRaw": []
}"#;
