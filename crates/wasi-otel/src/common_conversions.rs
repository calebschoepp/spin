use super::wasi::{self, clocks0_2_0::wall_clock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

impl From<wasi::otel::types::KeyValue> for opentelemetry::KeyValue {
    fn from(kv: wasi::otel::types::KeyValue) -> Self {
        opentelemetry::KeyValue::new(kv.key, kv.value)
    }
}

impl From<&wasi::otel::types::KeyValue> for opentelemetry::KeyValue {
    fn from(kv: &wasi::otel::types::KeyValue) -> Self {
        opentelemetry::KeyValue::new(kv.key.to_owned(), kv.value.to_owned())
    }
}

impl From<wasi::otel::types::Value> for opentelemetry::Value {
    fn from(value: wasi::otel::types::Value) -> Self {
        match value {
            wasi::otel::types::Value::String(v) => v.into(),
            wasi::otel::types::Value::Bool(v) => v.into(),
            wasi::otel::types::Value::F64(v) => v.into(),
            wasi::otel::types::Value::S64(v) => v.into(),
            wasi::otel::types::Value::StringArray(v) => opentelemetry::Value::Array(
                v.into_iter()
                    .map(opentelemetry::StringValue::from)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            wasi::otel::types::Value::BoolArray(v) => opentelemetry::Value::Array(v.into()),
            wasi::otel::types::Value::F64Array(v) => opentelemetry::Value::Array(v.into()),
            wasi::otel::types::Value::S64Array(v) => opentelemetry::Value::Array(v.into()),
        }
    }
}

impl From<wasi::otel::types::InstrumentationScope> for opentelemetry::InstrumentationScope {
    fn from(value: wasi::otel::tracing::InstrumentationScope) -> Self {
        let builder =
            Self::builder(value.name).with_attributes(value.attributes.into_iter().map(Into::into));
        match (value.version, value.schema_url) {
            (Some(version), Some(schema_url)) => builder
                .with_version(version)
                .with_schema_url(schema_url)
                .build(),
            (Some(version), None) => builder.with_version(version).build(),
            (None, Some(schema_url)) => builder.with_schema_url(schema_url).build(),
            (None, None) => builder.build(),
        }
    }
}

impl From<wall_clock::Datetime> for SystemTime {
    fn from(timestamp: wall_clock::Datetime) -> Self {
        UNIX_EPOCH
            + Duration::from_secs(timestamp.seconds)
            + Duration::from_nanos(timestamp.nanoseconds as u64)
    }
}
