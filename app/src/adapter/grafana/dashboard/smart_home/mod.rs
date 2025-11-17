use std::sync::Arc;

use crate::core::HomeApi;
use actix_web::web;

mod overview;

pub fn routes(api: Arc<HomeApi>) -> actix_web::Scope {
    web::scope("/smart_home").service(overview::routes(api.clone()))
}
