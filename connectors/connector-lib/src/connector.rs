use crate::oracle_core::{get_core_api_port, OracleCore};
use anyhow::Result;
use sincere;
use std::thread;
use std::time::Duration;

type OracleCorePort = String;
type Datapoint = u64;

pub struct Connector {
    pub title: String,
    pub description: String,
    pub get_datapoint: fn() -> Result<Datapoint>,
    pub print_info: fn(&Connector, &OracleCore) -> Result<bool>,
    pub start_api_server: fn(OracleCorePort),
    pub oracle_core_port: String,
}

// Key Connector methods
impl Connector {
    /// Create a new custom Connector
    pub fn new(
        title: &str,
        description: &str,
        get_datapoint: fn() -> Result<u64>,
        print_info: fn(&Connector, &OracleCore) -> Result<bool>,
        start_api_server: fn(String),
    ) -> Connector {
        let core_port =
            get_core_api_port().expect("Failed to read port from local `oracle-config.yaml`.");
        Connector {
            title: title.to_string(),
            description: description.to_string(),
            get_datapoint: get_datapoint,
            print_info: print_info,
            start_api_server: start_api_server,
            oracle_core_port: core_port,
        }
    }

    // Run the Connector using a local Oracle Core
    pub fn run(&self) {
        let oc = OracleCore::new("0.0.0.0", &self.oracle_core_port);

        // Main Loop
        loop {
            // If printing isn't successful (which involves fetching state from core)
            if let Err(e) = (self.print_info)(&self, &oc) {
                print!("\x1B[2J\x1B[1;1H");
                println!("Error: {:?}", e);
            }
            // Otherwise if state is accessible
            else {
                let pool_status = oc.pool_status().unwrap();
                let oracle_status = oc.oracle_status().unwrap();

                // Check if Connector should post
                let should_post = &pool_status.current_pool_stage == "Live Epoch"
                    && oracle_status.waiting_for_datapoint_submit;

                if should_post {
                    let price_res = (self.get_datapoint)();
                    // If acquiring price worked
                    if let Ok(price) = price_res {
                        // If submitting Datapoint tx worked
                        let submit_result = oc.submit_datapoint(price);
                        if let Ok(tx_id) = submit_result {
                            println!("\nSubmit New Datapoint: {} nanoErg/USD", price);
                            println!("Transaction ID: {}", tx_id);
                        } else {
                            println!("Datapoint Tx Submit Error: {:?}", submit_result);
                        }
                    } else {
                        println!("{:?}", price_res);
                    }
                }
            }

            thread::sleep(Duration::new(30, 0))
        }
    }
}

// Methods for setting up a default Basic Connector
impl Connector {
    /// Create a new basic Connector with a number of predefined defaults
    pub fn new_basic_connector(
        title: &str,
        description: &str,
        get_datapoint: fn() -> Result<u64>,
    ) -> Connector {
        Connector::new(
            title,
            description,
            get_datapoint,
            Connector::basic_print_info,
            Connector::basic_start_api_server,
        )
    }

    // Default Basic Connector print info
    fn basic_print_info(&self, oc: &OracleCore) -> Result<bool> {
        let pool_status = oc.pool_status()?;
        let oracle_status = oc.oracle_status()?;
        print!("\x1B[2J\x1B[1;1H");
        println!("{} Connector", self.title);
        println!("===========================================");
        println!("Current Blockheight: {}", oc.current_block_height()?);
        println!(
            "Current Oracle Pool Stage: {}",
            pool_status.current_pool_stage
        );
        println!(
            "Submit Datapoint In Latest Epoch: {}",
            !oracle_status.waiting_for_datapoint_submit
        );

        println!("Latest Datapoint: {}", oracle_status.latest_datapoint);
        println!("===========================================");
        Ok(true)
    }

    // Default Basic Connector api server
    fn basic_start_api_server(core_port: String) {
        let mut app = sincere::App::new();

        // Basic welcome endpoint
        app.get("/", move |context| {
        let response_text = format!(
            "This is an Oracle Core Connector. Please use one of the endpoints to interact with it.\n"
        );
        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_text(response_text)
            .unwrap();
    });

        // Start the API server with the port designated in the oracle config
        // plus two.
        let port = ((core_port
            .parse::<u16>()
            .expect("Failed to parse oracle core port from config to u16."))
            + 2)
        .to_string();
        let address = "0.0.0.0:".to_string() + &port;
        app.run(&address, 1).ok();
    }
}
