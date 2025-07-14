use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::StreamableHttpService;
use server::SmartHomeMcp;

use crate::core::HomeApi;

mod server;

pub fn new_routes(api: HomeApi) -> actix_web::Scope {
    let service = Arc::new(StreamableHttpService::new(
        move || Ok(SmartHomeMcp::new(api.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    ));

    let mcp_scope = StreamableHttpService::scope(service);

    actix_web::web::scope("/mcp").service(mcp_scope)
}
