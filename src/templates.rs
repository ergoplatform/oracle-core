



static COMMIT_ORACLE_DATAPOINT_REQUEST : &'static str = r#"{
  "requests": [
    {
      "address": "{{address}}",
      "value": 5000000,
      "assets": [
        {
          "tokenId": "{{token_id}}",
          "amount": 1
        }
      ],
      "registers": {
        "R4": "{{R4}}",
        "R5": "{{R5}}",
        "R6": "{{R6}}"
      }
    }
  ],
  "fee": 1000000,
  "inputsRaw": [ 
    "{{ergs_raw_input}}",
    "{{datapoint_box_raw_input}}"
  ]
}"#;