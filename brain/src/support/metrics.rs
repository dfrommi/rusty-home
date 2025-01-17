use monitoring::meter::increment;
use support::InternalId;

const ITEM_TYPE: &str = "item_type";
const ITEM_NAME: &str = "item_name";
const OPERATION: &str = "operation";

pub fn cache_hit_data_point_access(item: impl Into<support::InternalId>) {
    let int_id: InternalId = item.into();
    increment(
        "home_cache_hit",
        &[
            (OPERATION, "data_point_access"),
            (ITEM_TYPE, int_id.type_.as_str()),
            (ITEM_NAME, int_id.name.as_str()),
        ],
    );
}

pub fn cache_miss_data_point_access(item: impl Into<support::InternalId>) {
    let int_id: InternalId = item.into();

    increment(
        "home_cache_miss",
        &[
            (OPERATION, "data_point_access"),
            (ITEM_TYPE, int_id.type_.as_str()),
            (ITEM_NAME, int_id.name.as_str()),
        ],
    );
}
