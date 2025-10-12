#![allow(dead_code)]
use infrastructure::meter::increment;

const TAG_ID: &str = "tag_id";
const OPERATION: &str = "operation";

pub fn cache_hit_data_point_access(tag_id: i64) {
    increment(
        "home_cache_hit",
        &[(OPERATION, "data_point_access"), (TAG_ID, &tag_id.to_string())],
    );
}

pub fn cache_miss_data_point_access(tag_id: i64) {
    increment(
        "home_cache_miss",
        &[(OPERATION, "data_point_access"), (TAG_ID, &tag_id.to_string())],
    );
}
