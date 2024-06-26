use log::{LevelFilter, Metadata, Record};

pub fn init(boot_info: &'static boot::BootInfo) {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match boot_info.log_level {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    });

    info!("Current log level: {}", log::max_level());

    info!("Logger Initialized.");
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        match record.level() {
            log::Level::Error => println_warn!(
                "[E] {}@{}: {}",
                record.file_static().unwrap_or(""),
                record.line().unwrap_or(0),
                record.args()
            ),
            log::Level::Warn => println_warn!("[!] {}", record.args()),
            log::Level::Info => println!("[+] {}", record.args()),
            log::Level::Debug => println_serial!("[D] {}", record.args()),
            log::Level::Trace => println_serial!("[T] {}", record.args()),
        }
    }

    fn flush(&self) {}
}
