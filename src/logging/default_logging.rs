use crate::config::Logging;

use crate::logging::error::LoggingError;
use std::fmt::Write as _;
use std::io;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::{
    EnvFilter,
    fmt::{
        self,
        format::{FormatEvent, FormatFields, Writer},
    },
    layer::SubscriberExt,
    registry::LookupSpan,
};

pub async fn init_logging_async(cfg: &Logging) -> Result<(), LoggingError> {
    // filter
    let filter = EnvFilter::try_new(&cfg.filter)
        .map_err(|e| LoggingError::InvalidFilter(format!("{}: {}", cfg.filter, e)))?;

    // colored stdout only, no file
    let stdout_layer = fmt::layer()
        .with_timer(SecondsTime)
        .with_ansi(true)
        .event_format(FlatFormatter)
        .with_writer(io::stdout);

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer);

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/* ---------------- formatter ---------------- */

struct FlatFormatter;

impl<S, N> FormatEvent<S, N> for FlatFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &fmt::FmtContext<'_, S, N>,
        mut out: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        // timestamp up to seconds
        SecondsTime.format_time(&mut out)?;
        write!(out, " ")?;

        // level
        match *event.metadata().level() {
            Level::ERROR => write!(out, "\x1b[31mERROR\x1b[0m ")?,
            Level::WARN => write!(out, "\x1b[33mWARN \x1b[0m ")?,
            Level::INFO => write!(out, "\x1b[32mINFO \x1b[0m ")?,
            Level::DEBUG => write!(out, "\x1b[34mDEBUG\x1b[0m ")?,
            Level::TRACE => write!(out, "\x1b[90mTRACE\x1b[0m ")?,
        }

        // span fields
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                if let Some(fields) = span.extensions().get::<SpanFields>() {
                    write!(out, "{} ", fields.0)?;
                }
            }
        }

        // event fields + message
        ctx.field_format().format_fields(out.by_ref(), event)?;
        writeln!(out)
    }
}

/* ---------------- span fields ---------------- */

#[derive(Default)]
struct SpanFields(String);

struct SpanVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for SpanVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let _ = write!(self.0, "{}={} ", field.name(), value);
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _ = write!(self.0, "{}={:?} ", field.name(), value);
    }
}

impl tracing_subscriber::Layer<tracing_subscriber::Registry> for FlatFormatter {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, tracing_subscriber::Registry>,
    ) {
        let span = ctx.span(id).expect("span exists");
        let mut buf = String::new();
        attrs.record(&mut SpanVisitor(&mut buf));
        span.extensions_mut().insert(SpanFields(buf));
    }
}

/* ---------------- time ---------------- */

use chrono::{Datelike, Local, Timelike};

struct SecondsTime;

impl tracing_subscriber::fmt::time::FormatTime for SecondsTime {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = Local::now(); // local time
        write!(
            w,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        )
    }
}
