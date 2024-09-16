use crate::planning::do_plan;

use api::HomeApi;
use core::time;
use settings::Settings;
use sqlx::postgres::PgListener;
use std::sync::{Arc, OnceLock};

mod error;
mod planning;
mod prelude;
mod settings;
mod support;
mod thing;

pub use crate::error::{Error, Result};
use support::*;

static HOME_API_INSTANCE: OnceLock<HomeApi> = OnceLock::new();

pub fn home_api() -> &'static HomeApi {
    HOME_API_INSTANCE
        .get()
        .expect("Global home-api instance accessed before initialization")
}

pub fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    unsafe { std::env::set_var("RUST_LOG", "warn,brain=debug") };
    tracing_subscriber::fmt::init();

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap(),
    );

    let db_pool = rt
        .block_on(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(4)
                .connect(&settings.database.url),
        )
        .unwrap();

    rt.spawn(async move {
        let mut listener = PgListener::connect(&settings.database.url).await.unwrap();
        listener.listen("thing_values_insert").await.unwrap();

        while let Ok(notification) = listener.recv().await {
            tracing::info!("PG Received notification: {}", notification.payload());
        }
    });

    HOME_API_INSTANCE
        .set(HomeApi::new(db_pool, rt.clone()))
        .expect("Error setting global event bus instance");

    loop {
        tracing::info!("Start planning");
        do_plan();
        tracing::info!("Planning done");
        std::thread::sleep(time::Duration::from_secs_f64(30.0));
    }
}
