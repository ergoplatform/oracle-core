use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::Appender;
use log4rs::config::Root;
use log4rs::Config;

pub fn setup_log() {
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
                // TODO: read log level from environment variable or config file
                .build(LevelFilter::Info),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    log_panics::init();
}
