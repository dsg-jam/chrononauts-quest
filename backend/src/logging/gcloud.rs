use std::fmt::Display;
use std::{fmt, io};

use chrono::Utc;
use serde::ser::SerializeMap;
use serde::Serializer as _;
use serde_json::Serializer;
use tracing::{Level, Metadata, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::fmt::{FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct Format;

impl<S, N> FormatEvent<S, N> for Format
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let timestamp = Utc::now();

        let normalized_meta = event.normalized_metadata();
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        let mut visit = || {
            let mut ser = Serializer::new(WriteAdaptor::new(&mut writer));
            let mut ser = ser.serialize_map(None)?;
            ser.serialize_entry("time", &timestamp)?;
            ser.serialize_entry("severity", map_to_severity(*meta.level()))?;

            let mut visitor = tracing_serde::SerdeMapVisitor::new(ser);
            event.record(&mut visitor);
            ser = visitor.take_serializer()?;

            ser.serialize_entry("logging.googleapis.com/labels", &LabelAdapter(meta))?;

            ser.serialize_entry(
                "logging.googleapis.com/sourceLocation",
                &SourceLocationAdapter(meta),
            )?;
            let current_span = event
                .parent()
                .and_then(|id| ctx.span(id))
                .or_else(|| ctx.lookup_current());
            if let Some(span) = current_span {
                ser.serialize_entry(
                    "logging.googleapis.com/spanId",
                    &SerdeDisplayAdapter(span.id().into_u64()),
                )?;
            }

            ser.end()
        };
        visit().map_err(|_| fmt::Error)?;
        writeln!(writer)
    }
}

const fn map_to_severity(level: Level) -> &'static str {
    match level {
        Level::TRACE => "DEBUG",
        Level::DEBUG => "INFO",
        Level::INFO => "NOTICE",
        Level::WARN => "WARNING",
        Level::ERROR => "ERROR",
    }
}

struct SourceLocationAdapter<'a>(&'a Metadata<'a>);

impl<'a> serde::Serialize for SourceLocationAdapter<'a> {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let meta = &self.0;
        let mut ser = ser.serialize_map(None)?;

        if let Some(file) = meta.file() {
            ser.serialize_entry("file", file)?;
        }

        if let Some(line) = meta.line() {
            ser.serialize_entry("line", &SerdeDisplayAdapter(line))?;
        }

        ser.end()
    }
}

struct LabelAdapter<'a>(&'a Metadata<'a>);

impl<'a> serde::Serialize for LabelAdapter<'a> {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let meta = &self.0;
        let mut ser = ser.serialize_map(None)?;

        ser.serialize_entry("log_target", meta.target())?;

        ser.end()
    }
}

struct SerdeDisplayAdapter<T>(T);

impl<T: Display> serde::Serialize for SerdeDisplayAdapter<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn fmt::Write,
}

impl<'a> WriteAdaptor<'a> {
    pub fn new(fmt_write: &'a mut dyn fmt::Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
