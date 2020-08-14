use anyhow::Result;
use sincere;

type OracleCorePort = String;
type Datapoint = u64;

struct Connector {
    title: String,
    description: String,
    get_datapoint: fn() -> Result<Datapoint>,
    print_info: fn() -> Result<bool>,
    start_api_server: fn(OracleCorePort),
}

impl Connector {
    /// Create a new custom Connector
    pub fn new(
        title: String,
        description: String,
        get_datapoint: fn() -> Result<u64>,
        print_info: fn() -> Result<bool>,
        start_api_server: fn(String),
    ) -> Connector {
        Connector {
            title: title,
            description: description,
            get_datapoint: get_datapoint,
            print_info: print_info,
            start_api_server: start_api_server,
        }
    }
}

// Methods for setting up a default Basic Connector
impl Connector {
    /// Create a new basic Connector with a number of predefined defaults
    pub fn new_basic_connector(
        title: String,
        description: String,
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
    fn basic_print_info() -> Result<bool> {
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
