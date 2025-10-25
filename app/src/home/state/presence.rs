use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Estimatable, algo},
        },
    },
    port::{DataFrameAccess, DataPointAccess},
};
use r#macro::{EnumVariants, Id, trace_state};

//TODO impl anyoneSleeping. Requires impl of enum from crate

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    BedDennis,
    BedSabine,
    CouchLeft,
    CouchCenter,
    CouchRight,
}

impl Estimatable for Presence {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        algo::last_seen(at, df)
    }
}

impl DataPointAccess<Presence> for Presence {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Presence> for Presence {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        api.get_data_frame(self, range).await
    }
}

impl Presence {
    pub async fn away(api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        let (dennis_home, sabine_home) = tokio::try_join!(
            Presence::AtHomeDennis.current_data_point(api),
            Presence::AtHomeSabine.current_data_point(api)
        )?;

        let is_away = !dennis_home.value && !sabine_home.value;

        Ok(DataPoint::new(
            is_away,
            std::cmp::max(dennis_home.timestamp, sabine_home.timestamp),
        ))
    }
}
