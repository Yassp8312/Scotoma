use wasm_bindgen::prelude::*;

#[macro_export]
macro_rules! console_error {
    ($lit:literal) => {
        $crate::for_generated::web_utils::js_console_error($lit)
    };
    ($($tt:tt)*) => {
        $crate::for_generated::web_utils::js_console_error(&format!($($tt)*))
    };
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = "log")]
    pub fn js_console_log(msg: &str);

    #[wasm_bindgen(js_namespace = console, js_name = "error")]
    pub fn js_console_error(msg: &str);
}

const SCRIPT_PATH_GLOBAL: &str = "__SCOTOMA_FRB_SCRIPT_URL__";

/// Returns the reviewed script URL installed by SCOTOMA before Wasm starts.
///
/// Upstream 2.13.0 extracts this value by evaluating JavaScript that throws and
/// parses a stack trace. That requires CSP `unsafe-eval`. The application and
/// every nested FRB worker instead install this immutable global explicitly.
pub(crate) fn script_path() -> Option<String> {
    let global = js_sys::global();
    js_sys::Reflect::get(&global, &JsValue::from_str(SCRIPT_PATH_GLOBAL))
        .ok()?
        .as_string()
}

#[cfg(feature = "log")]
#[derive(Clone, Copy)]
pub(crate) struct WebConsoleLogger;

#[cfg(feature = "log")]
static WEB_CONSOLE_LOGGER: WebConsoleLogger = WebConsoleLogger;

#[cfg(feature = "log")]
impl WebConsoleLogger {
    pub(crate) fn init(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_logger(&WEB_CONSOLE_LOGGER).map(|()| log::set_max_level(level))
    }
}

#[cfg(feature = "log")]
impl log::Log for WebConsoleLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            js_console_log(&format!("{} - {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}
