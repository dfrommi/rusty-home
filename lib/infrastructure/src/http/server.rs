use actix_web::{
    dev::Service as _,
    http::header::{HeaderName, HeaderValue},
    *,
};
use anyhow::Context as _;
use serde::Deserialize;

use crate::TraceContext;

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
            let mut app = App::new()
                //has to be before other middlewares to capture trace id
                .wrap_fn(|req, srv| {
                    let fut = srv.call(req);
                    async {
                        let mut res = fut.await?;

                        if let Some(ctx) = TraceContext::current()
                            && let Ok(trace_id) = HeaderValue::from_str(ctx.trace_id().as_str())
                        {
                            res.headers_mut().insert(HeaderName::from_static("trace-id"), trace_id);
                        }

                        Ok(res)
                    }
                })
                .wrap(tracing_actix_web::TracingLogger::default());

            //Add all scopes (top-level resources)
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
