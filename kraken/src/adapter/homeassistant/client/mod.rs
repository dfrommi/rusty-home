mod http;
mod mqtt;

//2 clients because mqtt recv need mut reference and so the client can't be used end parallel
pub use http::HaHttpClient;
pub use mqtt::HaMqttClient;
