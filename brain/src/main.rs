use crate::planning::do_plan;

use api::HomeApi;
use core::time;
use settings::Settings;
use sqlx::postgres::PgListener;
use std::sync::{Arc, OnceLock};
use tokio::task::JoinSet;

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

#[tokio::main]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    unsafe { std::env::set_var("RUST_LOG", "warn,brain=debug") };
    tracing_subscriber::fmt::init();

    let mut tasks = JoinSet::new();

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&settings.database.url)
        .await
        .expect("Error initializing database");

    tasks.spawn(async move {
        let mut listener = PgListener::connect(&settings.database.url).await.unwrap();
        listener.listen("thing_values_insert").await.unwrap();

        while let Ok(notification) = listener.recv().await {
            tracing::info!("PG Received notification: {}", notification.payload());
        }
    });

    HOME_API_INSTANCE
        .set(HomeApi::new(db_pool))
        .expect("Error setting global event bus instance");

    tasks.spawn(async {
        loop {
            tracing::info!("Start planning");
            do_plan().await;
            tracing::info!("Planning done");
            std::thread::sleep(time::Duration::from_secs_f64(30.0));
        }
    });

    while let Some(task) = tasks.join_next().await {
        let () = task.unwrap();
    }
}
