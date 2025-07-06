use infrastructure::meter::increment;

const ITEM_TYPE: &str = "item_type";
const ITEM_NAME: &str = "item_name";
const OPERATION: &str = "operation";

pub fn cache_hit_data_point_access(item: impl AsRef<crate::core::id::ExternalId>) {
    let ext_id: &crate::core::id::ExternalId = item.as_ref();

    increment(
        "home_cache_hit",
        &[
            (OPERATION, "data_point_access"),
            (ITEM_TYPE, ext_id.ext_type()),
            (ITEM_NAME, ext_id.ext_name()),
        ],
    );
}

pub fn cache_miss_data_point_access(item: impl AsRef<crate::core::id::ExternalId>) {
    let ext_id: &crate::core::id::ExternalId = item.as_ref();

    increment(
        "home_cache_miss",
        &[
            (OPERATION, "data_point_access"),
            (ITEM_TYPE, ext_id.ext_type()),
            (ITEM_NAME, ext_id.ext_name()),
        ],
    );
}
