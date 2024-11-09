pub use api::state::ExternalAutoControl;

use crate::{
    adapter::persistence::{DataPoint, StateRepository},
    home_api,
};

use super::DataPointAccess;

impl DataPointAccess<bool> for ExternalAutoControl {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<bool>> {
        home_api().get_latest(self).await
    }
}
