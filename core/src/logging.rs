use std::path::Path;

use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::Appender;
use log4rs::config::Logger;
use log4rs::config::Root;
use log4rs::Config;

pub fn setup_log(
    cmdline_log_level: Option<LevelFilter>,
    config_log_level: Option<LevelFilter>,
    data_dir: &Path,
) {
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

    let config_log_level = config_log_level.unwrap_or(LevelFilter::Info);
    let log_level = if let Some(cmdline_log_level) = cmdline_log_level {
        if cmdline_log_level > config_log_level {
            cmdline_log_level
        } else {
            config_log_level
        }
    } else {
        config_log_level
    };

    // cmdline_log_level.unwrap_or_else(get_level_filter);

    let log_path = data_dir.join("oracle-core.log");

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(
            Appender::builder().build(
                "logfile",
                Box::new(
                    RollingFileAppender::builder()
                        .build(log_path, Box::new(compound_policy))
                        .unwrap(),
                ),
            ),
        )
        .logger(
            Logger::builder()
                .appender("logfile")
                .appender("stdout")
                .additive(false)
                .build("oracle_core", log_level),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .build(LevelFilter::Info),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    log_panics::init();
}
