mod error;

use crate::config::Logging;
use std::fmt::Write as _;
use std::io;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    EnvFilter,
    fmt::{
        self,
        format::{FormatEvent, FormatFields, Writer},
        time::{FormatTime, LocalTime},
    },
    layer::SubscriberExt,
    registry::LookupSpan,
};

pub async fn init_logging_async(cfg: &Logging) -> anyhow::Result<()> {
    // filter
    let filter = EnvFilter::try_new(&cfg.filter)
        .map_err(|e| anyhow::anyhow!("invalid log filter '{}': {}", cfg.filter, e))?;

    // colored
    let stdout_layer = fmt::layer()
        .with_timer(LocalTime::rfc_3339())
        .with_ansi(true)
        .event_format(FlatFormatter)
        .with_writer(io::stdout);

    // plain
    let file_layer = cfg.file.clone().map(|path| {
        fmt::layer()
            .with_timer(LocalTime::rfc_3339())
            .with_ansi(false)
            .event_format(FlatFormatter)
            .with_writer(move || {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .expect("log file open failed")
            })
    });

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer);

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
        // timestamp
        LocalTime::rfc_3339().format_time(&mut out)?;
        write!(out, " ")?;

        // level
        match *event.metadata().level() {
            Level::ERROR => write!(out, "\x1b[31mERROR\x1b[0m ")?,
            Level::WARN => write!(out, "\x1b[33mWARN \x1b[0m ")?,
            Level::INFO => write!(out, "\x1b[32mINFO \x1b[0m ")?,
            Level::DEBUG => write!(out, "\x1b[34mDEBUG\x1b[0m ")?,
            Level::TRACE => write!(out, "\x1b[90mTRACE\x1b[0m ")?,
        }

        // span fields (server=main, conn_id, etc)
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                if let Some(fields) = span.extensions().get::<SpanFields>() {
                    write!(out, "{} ", fields.0)?;
                }
            }
        }

        // event fields + message (event name)
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
