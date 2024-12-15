use std::sync::OnceLock;

use sqlx::PgPool;
use support::time::{DateTime, FIXED_NOW};
use tokio::runtime::Runtime;

use crate::{
    planning::action::{Action, HomeAction},
    settings,
};

pub struct ActionState {
    pub is_fulfilled: bool,
}

pub fn get_state_at(iso: &str, action: impl Into<HomeAction>) -> ActionState {
    let fake_now = DateTime::from_iso(iso).unwrap();
    let action: HomeAction = action.into();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        let api = infrastructure();

        let is_fulfilled = action.preconditions_fulfilled(api).await.unwrap();

        ActionState { is_fulfilled }
    }))
}

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
