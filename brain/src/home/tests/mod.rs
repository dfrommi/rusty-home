mod action;
mod planning;

use std::sync::OnceLock;

use crate::{adapter::persistence::Database, settings};
use tokio::runtime::Runtime;

struct TestInfrastructure {
    runtime: Runtime,
    pool: sqlx::PgPool,
}

impl TestInfrastructure {
    pub fn api(&self) -> Database {
        //new instance to avoid caching
        Database::new(self.pool.clone())
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
            settings.live_database.new_pool().await.unwrap()
        });

        TestInfrastructure {
            runtime,
            pool: db_pool,
        }
    })
}
