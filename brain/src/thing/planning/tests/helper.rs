use std::sync::OnceLock;

use support::time::{DateTime, FIXED_NOW};
use tokio::runtime::Runtime;

use crate::{
    adapter::persistence::HomeApi,
    settings,
    thing::planning::action::{Action, HomeAction},
    HOME_API_INSTANCE,
};

pub struct ActionState {
    pub is_fulfilled: bool,
    pub is_running: bool,
}

pub fn get_state_at(iso: &str, action: impl Into<HomeAction>) -> ActionState {
    let fake_now = DateTime::from_iso(iso).unwrap();
    let action: HomeAction = action.into();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        //init_api().await;
        let (is_fulfilled, is_running) =
            tokio::try_join!(action.preconditions_fulfilled(), action.is_running()).unwrap();

        ActionState {
            is_fulfilled,
            is_running,
        }
    }))
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime");

        HOME_API_INSTANCE.get_or_init(|| runtime.block_on(create_api()));

        runtime
    })
}

async fn create_api() -> HomeApi {
    let settings = settings::test::TestSettings::load().unwrap();

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(settings.live_database.url.as_str())
        .await
        .unwrap();

    HomeApi::new(db_pool)
}
