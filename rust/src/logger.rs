use std::env;

lazy_static! {
    static ref CONSOLE_ENABLED: i8 = {
      let var_level = env::var("J4RS_CONSOLE_LOG_LEVEL").unwrap_or("info".to_owned()).to_lowercase();
      match var_level.as_str() {
        "disabled" => 0,
        "error" => 1,
        "warn" => 2,
        "info" => 3,
        "debug" => 4,
        _ => {
          println!("WARN: The env variable 'J4RS_CONSOLE_LOG_LEVEL' is not correctly set. Please use one of the 'debug', 'info', 'warn', 'error', or 'disabled'. Defaulting to info");
          3
        }
      }
    };
}

pub fn debug(message: &str) {
    if CONSOLE_ENABLED.to_owned() > 3 {
        println!("DEBUG: {}", message);
    }
    debug!("{}", message);
}

pub fn info(message: &str) {
    if CONSOLE_ENABLED.to_owned() > 2 {
        println!("INFO: {}", message);
    }
    info!("{}", message);
}

#[allow(dead_code)]
pub fn warn(message: &str) {
    if CONSOLE_ENABLED.to_owned() > 1 {
        println!("WARN: {}", message);
    }
    warn!("{}", message);
}

#[allow(dead_code)]
pub fn error(message: &str) {
    if CONSOLE_ENABLED.to_owned() > 0 {
        println!("ERROR: {}", message);
    }
    error!("{}", message);
}