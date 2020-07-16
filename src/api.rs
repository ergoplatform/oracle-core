use crate::oracle_config::PoolParameters;
use crate::oracle_state::OraclePool;
use crossbeam::channel;
use sincere;

/// Starts the API server
pub fn start_api() {
    let mut app = sincere::App::new();
    let parameters = PoolParameters::new();
    let op = OraclePool::new();

    app.get("/", move |context| {
        let response_text = format!(
            "This is an Oracle Core. Please use one of the endpoints to interact with it.\n"
        );
        context.response.from_text(response_text).unwrap();
    });

    app.run("0.0.0.0:9031", 5).unwrap();
}
