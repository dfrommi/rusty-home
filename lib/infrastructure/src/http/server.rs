use actix_web::*;
use anyhow::Context as _;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct HttpServerConfig {
    pub port: u16,
}

impl HttpServerConfig {
    pub async fn run_server<F>(&self, scopes: F) -> anyhow::Result<()>
    where
        F: Fn() -> Vec<Scope> + Send + Clone + 'static,
    {
        let http_server = HttpServer::new(move || {
            let mut app = App::new().wrap(tracing_actix_web::TracingLogger::default());

            for scope in scopes() {
                app = app.service(scope);
            }

            app
        })
        .workers(1)
        .disable_signals()
        .bind(("0.0.0.0", self.port))?;

        http_server
            .run()
            .await
            .with_context(|| format!("Error starting HTTP server on port {}", self.port))
    }
}
