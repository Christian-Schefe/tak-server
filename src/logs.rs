use log::LevelFilter;
use log4rs::{
    Config,
    append::{
        console::{ConsoleAppender, Target},
        rolling_file::policy::compound::{
            CompoundPolicy, roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
        },
    },
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};

const LOG_SIZE_LIMIT: u64 = 10 * 1024 * 1024; // 10 MB

const LOG_FILE_COUNT: u32 = 3;

pub fn init_logger() {
    let file_path = std::env::var("LOG_FILE_PATH").expect("LOG_FILE_PATH must be set");
    let archive_pattern =
        std::env::var("LOG_ARCHIVE_PATTERN").expect("LOG_ARCHIVE_PATTERN must be set");

    let stderr_level = LevelFilter::Info;
    let file_level = LevelFilter::Debug;

    let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

    let trigger = SizeTrigger::new(LOG_SIZE_LIMIT);
    let roller = FixedWindowRoller::builder()
        .build(&archive_pattern, LOG_FILE_COUNT)
        .unwrap();
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let logfile = log4rs::append::rolling_file::RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(file_path, Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(file_level)))
                .build("logfile", Box::new(logfile)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(stderr_level)))
                .build("stderr", Box::new(stderr)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .appender("stderr")
                .build(LevelFilter::Trace),
        )
        .unwrap();

    let _handle = log4rs::init_config(config).expect("Failed to initialize logger");
}
