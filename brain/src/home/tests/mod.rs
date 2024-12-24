mod action;
mod planning;

use std::sync::OnceLock;

use crate::settings;
use sqlx::PgPool;
use tokio::runtime::Runtime;

struct TestInfrastructure {
    runtime: Runtime,
    db_pool: PgPool,
}

impl AsRef<PgPool> for TestInfrastructure {
    fn as_ref(&self) -> &PgPool {
        &self.db_pool
    }
}

static INFRASTRUCTURE: OnceLock<TestInfrastructure> = OnceLock::new();

fn runtime() -> &'static Runtime {
    &infrastructure().runtime
}

fn infrastructure() -> &'static TestInfrastructure {
    INFRASTRUCTURE.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime");

        let db_pool = runtime.block_on(async {
            let settings = settings::test::TestSettings::load().unwrap();
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(4)
                .connect(settings.live_database.url.as_str())
                .await
                .unwrap()
        });

        TestInfrastructure { runtime, db_pool }
    })
}
