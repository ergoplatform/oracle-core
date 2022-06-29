use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::Appender;
use log4rs::config::Root;
use log4rs::Config;

use crate::oracle_config::MAYBE_ORACLE_CONFIG;

fn load_log_level() -> Option<LevelFilter> {
    MAYBE_ORACLE_CONFIG.clone().ok()?.log_level
    // let config_file = std::fs::read_to_string(oracle_config::DEFAULT_CONFIG_FILE_NAME).ok()?;
    // YamlLoader::load_from_str(&config_file).ok()?.first()?["log_level"]
    //     .as_str()
    //     .map(|s| s.to_string())
}

fn get_level_filter() -> LevelFilter {
    load_log_level().unwrap_or(LevelFilter::Info)
    // let log_level = load_log_level().unwrap_or_else(|| "info".to_string());
    // match log_level.to_lowercase().as_str() {
    //     "trace" => LevelFilter::Trace,
    //     "debug" => LevelFilter::Debug,
    //     "info" => LevelFilter::Info,
    //     "warn" => LevelFilter::Warn,
    //     "error" => LevelFilter::Error,
    //     "off" => LevelFilter::Off,
    //     _ => LevelFilter::Info,
    // }
}

pub fn setup_log(override_log_level: Option<LevelFilter>) {
    let stdout = ConsoleAppender::builder().build();

    // via https://stackoverflow.com/questions/56345288/how-do-i-use-log4rs-rollingfileappender-to-incorporate-rolling-logging#
    let window_size = 3; // log0, log1, log2
    let fixed_window_roller = FixedWindowRoller::builder()
        .build("oracle-core.log{}", window_size)
        .unwrap();

    let size_limit = 5 * 1024 * 1024; // 5MB as max log file size to roll
    let size_trigger = SizeTrigger::new(size_limit);

    let compound_policy =
        CompoundPolicy::new(Box::new(size_trigger), Box::new(fixed_window_roller));

    let log_level = override_log_level.unwrap_or_else(get_level_filter);

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(
            Appender::builder().build(
                "logfile",
                Box::new(
                    RollingFileAppender::builder()
                        .build("oracle-core.log", Box::new(compound_policy))
                        .unwrap(),
                ),
            ),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .build(log_level),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    log_panics::init();
}
