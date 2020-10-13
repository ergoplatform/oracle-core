use crate::oracle_core::{get_core_api_port, OracleCore};
use anyhow::Result;
use ergo_lib::ast::Constant;
use std::env;
use std::thread;
use std::time::Duration;

pub type Datapoint = u64;

#[derive(Clone)]
pub struct Connector {
    pub title: String,
    pub get_datapoint: fn() -> Result<Datapoint>,
    pub print_info: fn(&Connector, &OracleCore) -> Result<bool>,
}

// Key Connector methods
impl Connector {
    /// Create a new custom Connector
    pub fn new(
        title: &str,
        get_datapoint: fn() -> Result<u64>,
        print_info: fn(&Connector, &OracleCore) -> Result<bool>,
    ) -> Connector {
        let connector = Connector {
            title: title.to_string(),
            get_datapoint: get_datapoint,
            print_info: print_info,
        };
        connector.check_bootstrap();
        connector
    }

    /// Checks if asked for bootstrap value via CLI flag
    pub fn check_bootstrap(&self) {
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 && &args[1] == "--bootstrap-value" {
            if let Ok(price) = (self.get_datapoint)() {
                // Convert the price into a sigma `Constant`
                let constant: Constant = (price as i64).into();

                println!("Bootstrap {} Value: {}", self.title, constant.base16_str());
                std::process::exit(0);
            } else {
                panic!("Failed to fetch Connector bootstrap value.");
            }
        }
    }

    /// Run the Connector using a local Oracle Core
    pub fn run(&self) {
        let core_port =
            get_core_api_port().expect("Failed to read port from local `oracle-config.yaml`.");
        let oc = OracleCore::new("0.0.0.0", &core_port);

        // Main Loop
        loop {
            // If printing isn't successful (which involves fetching state from core)
            if let Err(e) = (self.print_info)(&self, &oc) {
                print!("\x1B[2J\x1B[1;1H");
                println!("Error: {:?}", e);
            }
            // Otherwise if state is accessible
            else {
                // Guarantee that the connector won't panic
                let res_ps = oc.pool_status();
                let res_os = oc.oracle_status();
                if let Err(e) = res_ps {
                    print!("\x1B[2J\x1B[1;1H");
                    println!("Error: {:?}", e);
                    continue;
                }
                if let Err(e) = res_os {
                    print!("\x1B[2J\x1B[1;1H");
                    println!("Error: {:?}", e);
                    continue;
                }
                let pool_status = res_ps.unwrap();
                let oracle_status = res_os.unwrap();

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
                            println!("\nSubmit New {} Datapoint: {}", self.title, price);
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
    pub fn new_basic_connector(title: &str, get_datapoint: fn() -> Result<u64>) -> Connector {
        Connector::new(title, get_datapoint, Connector::basic_print_info)
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
}
