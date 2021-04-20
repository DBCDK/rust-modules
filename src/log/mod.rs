
use std::cell::RefCell;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::Serialize;
use serde_json::json;
use std::option::Option;
use std::default::Default;
use uuid::Uuid;
use std::fmt::Debug;
use chrono::prelude::*;
use once_cell::sync::OnceCell;

#[derive(Serialize, Debug)]
pub enum LogLevel {
    INFO,
    ERROR
}

#[derive(Default)]
pub struct LogEntry<'se> {
    pub level: LogLevel,
    pub message: &'se str,
    pub data: Option<&'se dyn erased_serde::Serialize>,
    pub error: Option<&'se dyn Debug>,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::INFO
    }
}

pub struct LogConfig {
    pub get_timestamp: Box<dyn Fn() -> DateTime<Utc>>,
    pub print: Box<dyn Fn(String)>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self{
            get_timestamp: Box::new(|| Utc::now()),
            print: Box::new(|msg| println!("{}", msg))
        }
    }
}

static APP_NAME: OnceCell<String> = OnceCell::new();

thread_local! {
    static CONFIG: RefCell<LogConfig> = RefCell::new(LogConfig::default());
    static SESSION_ID: RefCell<Uuid> = RefCell::new(Uuid::nil());
}

pub fn init(app_name: String) -> Result<(), String> {
    APP_NAME.set(app_name)
}

pub fn thread_configure(config: LogConfig) {
    clear_session();
    CONFIG.with(|conf| {
        *conf.borrow_mut() = config
    });
}

pub fn new_session() -> Uuid {
    let id = Uuid::new_v4();
    set_session(id.clone());
    id
}

pub fn enter_session(id: &Uuid) {
    set_session(id.clone());
}

pub fn clear_session() {
    set_session(Uuid::nil());
}

fn set_session(id: Uuid) {
    SESSION_ID.with(|uuid| {
        *uuid.borrow_mut() = id;
    })
}

impl Serialize for LogEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        
        //2016-07-25T17:22:40.835692521+02:00, 2016-07-25T17:22:40.835Z
        CONFIG.with(|conf_| {
            let conf = conf_.borrow();
            let app_name = APP_NAME.get()
                .map(std::borrow::ToOwned::to_owned)
                .unwrap_or(env!("CARGO_PKG_NAME").to_string());

            let mut s = serializer.serialize_struct("LogEntry", 8)?;
            s.serialize_field("app", &app_name)?;
            if self.data.is_some() {
                s.serialize_field("data", &json!({ app_name: &self.data.unwrap() }))?;
            }
            if self.error.is_some() {
                s.serialize_field("error", &format!("{:?}", self.error.unwrap()))?;
            }
            s.serialize_field("level", &self.level)?;
            s.serialize_field("message", &self.message)?;
            s.serialize_field("sessionid", &SESSION_ID.with(|uuid| {
                uuid.borrow().to_string()
            }))?;
            s.serialize_field("timestamp", &(conf.get_timestamp)().to_rfc3339_opts(SecondsFormat::Millis, true))?;
            s.end()
        })
    }
}

fn log(entry: LogEntry) {
    CONFIG.with(|conf| (conf.borrow().print)(format!("{}", serde_json::to_string(&entry).unwrap())));
}


pub fn info(message: &str) {
    log(LogEntry{
        level: LogLevel::INFO,
        message,
        ..Default::default()
    });
}

pub fn data<T>(message: &str, data: &T) where T: Serialize {
    log(LogEntry{
        level: LogLevel::INFO,
        message,
        data: Some(data),
        ..Default::default()
    });
}

pub fn error<E>(message: &str, error: &E) where E: Debug {
    log(LogEntry{
        level: LogLevel::ERROR,
        message,
        error: Some(error),
        ..Default::default()
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_logging_without_session() {
        thread_local! {
            static OUTPUT: RefCell<String> = RefCell::new(String::new());
        }
        super::thread_configure(LogConfig{
            get_timestamp: Box::new(|| Utc.ymd(2014, 7, 8).and_hms(9, 10, 11)),
            print: Box::new(|msg| OUTPUT.with(|output| *output.borrow_mut() = msg)),
            ..Default::default()
        });
        super::info("test logging without session");

        OUTPUT.with(|output| {
            assert_eq!(json!({
                "timestamp": "2014-07-08T09:10:11.000Z",
                "app": "dbc-rust-modules",
                "level": "INFO",
                "message": "test logging without session",
                "sessionid": "00000000-0000-0000-0000-000000000000"
            }).to_string(), (output.borrow()).as_str());
        });
    }

    #[test]
    fn test_logging_with_session() {
        thread_local! {
            static OUTPUT: RefCell<String> = RefCell::new(String::new());
        }
        super::thread_configure(LogConfig{
            get_timestamp: Box::new(|| Utc.ymd(2014, 7, 8).and_hms(9, 10, 11)),
            print: Box::new(|msg| OUTPUT.with(|output| *output.borrow_mut() = msg)),
            ..Default::default()
        });

        let session_id = Uuid::new_v4();
        super::enter_session(&session_id);
        super::info("test logging with session");

        OUTPUT.with(|output| {
            assert_eq!(json!({
                "timestamp": "2014-07-08T09:10:11.000Z",
                "app": "dbc-rust-modules",
                "level": "INFO",
                "message": "test logging with session",
                "sessionid": session_id.to_string()
            }).to_string(), (output.borrow()).as_str());
        });
    }
}