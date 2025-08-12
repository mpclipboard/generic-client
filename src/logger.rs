pub struct Logger;

impl Logger {
    pub fn init() {
        #[cfg(target_os = "android")]
        {
            use android_logger::Config;
            use log::LevelFilter;

            android_logger::init_once(
                Config::default()
                    .with_tag("RUST")
                    .with_max_level(LevelFilter::Trace),
            );
        }

        #[cfg(not(target_os = "android"))]
        pretty_env_logger::init();
    }

    pub fn test() {
        log::info!("info example");
        log::error!("error example");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_logger_test() {
    Logger::test();
}
