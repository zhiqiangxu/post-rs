use std::ffi::c_char;

use log::{Level, Log, Metadata, Record};

/// FFI-safe borrowed Rust &str
#[repr(C)]
pub struct StringView {
    pub ptr: *const c_char,
    pub len: usize,
}

impl<'a> From<&'a str> for StringView {
    fn from(s: &'a str) -> Self {
        Self {
            ptr: s.as_ptr() as _,
            len: s.len(),
        }
    }
}

#[repr(C)]
pub struct ExternCRecord {
    pub level: Level,
    pub message: StringView,
    pub module_path: StringView,
    pub file: StringView,
    pub line: i64,
}

struct ExternCLog {
    callback: extern "C" fn(&ExternCRecord),
    level: Level,
}

impl ExternCLog {
    fn new(level: Level, callback: extern "C" fn(&ExternCRecord)) -> Self {
        Self { level, callback }
    }
}

impl Log for ExternCLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        let msg = record.args().to_string();
        let c_record = ExternCRecord {
            level: record.level(),
            message: StringView::from(msg.as_str()),
            module_path: StringView::from(record.module_path().unwrap_or("")),
            file: StringView::from(record.file().unwrap_or("")),
            line: record.line().map(|u| u as i64).unwrap_or(-1_i64),
        };

        (self.callback)(&c_record);
    }

    fn flush(&self) {}
}

/// Set a logging callback function
/// The function is idempotent, calling it more then once will have no effect.
/// Returns 0 if the callback was set successfully, 1 otherwise.
#[no_mangle]
pub extern "C" fn set_logging_callback(
    level: Level,
    callback: extern "C" fn(&ExternCRecord),
) -> i32 {
    match log::set_boxed_logger(Box::new(ExternCLog::new(level, callback)))
        .map(|()| log::set_max_level(level.to_level_filter()))
    {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Failed to set logger ({e})");
            1
        }
    }
}
