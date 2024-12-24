mod command;
mod planning_trace;
mod state;
mod trigger;

#[cfg(test)]
#[derive(derive_more::AsRef)]
struct TestDb {
    pool: sqlx::PgPool,
}
