use monitoring::meter::increment;

const ITEM_TYPE: &str = "item_type";
const ITEM_NAME: &str = "item_name";
const OPERATION: &str = "operation";

pub fn cache_hit_data_point_access(item: &impl support::TypedItem) {
    increment(
        "home_cache_hit",
        &[
            (OPERATION, "data_point_access"),
            (ITEM_TYPE, item.type_name()),
            (ITEM_NAME, item.item_name()),
        ],
    );
}

pub fn cache_miss_data_point_access(item: &impl support::TypedItem) {
    increment(
        "home_cache_miss",
        &[
            (OPERATION, "data_point_access"),
            (ITEM_TYPE, item.type_name()),
            (ITEM_NAME, item.item_name()),
        ],
    );
}
