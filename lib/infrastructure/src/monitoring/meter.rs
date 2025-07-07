use cached::proc_macro::cached;
use opentelemetry::KeyValue;

pub fn increment(name: &'static str, kv: &[(&str, &str)]) {
    let kv: Vec<KeyValue> = kv.iter().map(|(k, v)| as_kv(k, v)).collect();
    counter(name).add(1, &kv)
}

pub fn set(name: &'static str, value: f64, kv: &[(&str, &str)]) {
    let kv: Vec<KeyValue> = kv.iter().map(|(k, v)| as_kv(k, v)).collect();
    gauge(name).record(value, &kv)
}

fn as_kv(k: &str, v: &str) -> KeyValue {
    KeyValue::new(k.to_owned(), v.to_owned())
}

#[cached]
fn counter(name: &'static str) -> opentelemetry::metrics::Counter<u64> {
    opentelemetry::global::meter("home").u64_counter(name).build()
}

#[cached]
fn gauge(name: &'static str) -> opentelemetry::metrics::Gauge<f64> {
    opentelemetry::global::meter("home").f64_gauge(name).build()
}
