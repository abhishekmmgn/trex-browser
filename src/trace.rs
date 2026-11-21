//! KB_TRACE_SCROLL=<file>: a timed log of each wheel report from the
//! terminal, what was sent to Chrome for it, and each frame on its way to
//! the screen, for finding out why scrolling feels as it does. With it,
//! KB_TRACE_FRAMES=<dir> keeps every frame Chrome sends as a PNG.
//!
//! Each line starts with the milliseconds since kb started.

use std::fs::File;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

struct Trace {
    start: Instant,
    file: Mutex<File>,
}

fn trace() -> Option<&'static Trace> {
    static TRACE: OnceLock<Option<Trace>> = OnceLock::new();
    TRACE
        .get_or_init(|| {
            let path = std::env::var_os("KB_TRACE_SCROLL")?;
            let file = File::options().create(true).append(true).open(path).ok()?;
            Some(Trace { start: Instant::now(), file: Mutex::new(file) })
        })
        .as_ref()
}

/// Whether KB_TRACE_SCROLL is set.
pub fn on() -> bool {
    trace().is_some()
}

/// Milliseconds since kb started, as traced.
pub fn now() -> f64 {
    trace().map_or(0.0, |trace| trace.start.elapsed().as_secs_f64() * 1000.0)
}

/// Append one line, timed, if tracing.
pub fn line(text: std::fmt::Arguments) {
    if let Some(trace) = trace() {
        let at = trace.start.elapsed().as_secs_f64() * 1000.0;
        let _ = writeln!(trace.file.lock().unwrap(), "{at:10.2} {text}");
    }
}

macro_rules! scroll {
    ($($arg:tt)*) => {
        if $crate::trace::on() {
            $crate::trace::line(format_args!($($arg)*));
        }
    };
}
pub(crate) use scroll;
