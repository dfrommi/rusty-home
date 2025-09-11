use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use server::SmartHomeMcp;

use crate::core::HomeApi;

mod server;

pub fn new_routes(api: HomeApi) -> actix_web::Scope {
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(move || Ok(SmartHomeMcp::new(api.clone()))))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .build();

    let mcp_scope = StreamableHttpService::scope(service);

    actix_web::web::scope("/mcp").service(mcp_scope)
}
