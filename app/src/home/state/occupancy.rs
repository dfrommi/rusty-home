use crate::core::{
    HomeApi,
    math::Sigmoid,
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable, LastSeenInterpolator},
    },
};

use super::*;
use crate::port::DataFrameAccess;
use anyhow::Result;

use crate::core::math::DataFrameStatsExt as _;
use crate::core::time::{DateTime, DateTimeRange};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Occupancy {
    LivingRoomCouch,
    BedroomBed,
    RoomOfRequirementsDesk,
}

impl Occupancy {
    pub fn calculate_presence(mut presence: DataFrame<bool>) -> anyhow::Result<f64> {
        presence.retain_range(
            &DateTimeRange::since(t!(1 hours ago)),
            LastSeenInterpolator,
            LastSeenInterpolator,
        )?;

        let s1 = presence.weighted_aged_sum(t!(30 minutes), LastSeenInterpolator);
        Ok(s1)
    }

    pub fn calculate(prior: f64, w_presence: f64, presence: DataFrame<bool>) -> anyhow::Result<Probability> {
        let sigmoid = Sigmoid::default();
        let s1 = Self::calculate_presence(presence)?;

        //let prior = logit(p(0.03));

        Ok(sigmoid.eval(prior + w_presence * s1))
    }
}

impl Estimatable for Occupancy {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Probability>) -> Option<Probability> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<Probability> for Occupancy {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<Probability>> {
        let range = DateTimeRange::since(t!(1 hours ago));

        let main_df = match self {
            Occupancy::LivingRoomCouch => Presence::LivingRoomCouch.get_data_frame(range.clone(), api).await?,
            Occupancy::BedroomBed => Presence::BedroomBed.get_data_frame(range.clone(), api).await?,
            Occupancy::RoomOfRequirementsDesk => {
                IsRunning::RoomOfRequirementsMonitor
                    .get_data_frame(range.clone(), api)
                    .await?
            }
        };

        let prior: f64 = -1.7968470630447446;
        let w_presence: f64 = 3.733237448369802;

        let probability = Occupancy::calculate(prior, w_presence, main_df)?;

        Ok(DataPoint::new(probability, t!(now)))
    }
}

impl DataFrameAccess<Probability> for Occupancy {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<Probability>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfa::prelude::*;
    use linfa_linear::LinearRegression;
    use ndarray::array;

    use crate::core::{math::Sigmoid, timeseries::DataPoint};

    #[test]
    fn training() {
        let sigmoid = Sigmoid::default();

        let df0 = DataFrame::new(vec![DataPoint::new(true, t!(1 hours ago))]).unwrap();
        let df1 = DataFrame::new(vec![DataPoint::new(false, t!(1 hours ago))]).unwrap();
        let df2 = DataFrame::new(vec![
            DataPoint::new(false, t!(1 hours ago)),
            DataPoint::new(true, t!(15 minutes ago)),
            DataPoint::new(false, t!(2 minutes ago)),
            DataPoint::new(true, t!(1 minutes ago)),
        ])
        .unwrap();

        let df3 = DataFrame::new(vec![
            DataPoint::new(true, t!(1 hours ago)),
            DataPoint::new(false, t!(20 minutes ago)),
            DataPoint::new(true, t!(1 minutes ago)),
        ])
        .unwrap();
        let df4 = DataFrame::new(vec![
            DataPoint::new(false, t!(1 hours ago)),
            DataPoint::new(true, t!(10 minutes ago)),
        ])
        .unwrap();
        let df5 = DataFrame::new(vec![
            DataPoint::new(true, t!(1 hours ago)),
            DataPoint::new(false, t!(5 minutes ago)),
            DataPoint::new(true, t!(4 minutes ago)),
            DataPoint::new(false, t!(3 minutes ago)),
            DataPoint::new(true, t!(2 minutes ago)),
            DataPoint::new(false, t!(1 minutes ago)),
        ])
        .unwrap();

        let features = array![
            [Occupancy::calculate_presence(df0.clone()).unwrap()],
            [Occupancy::calculate_presence(df1.clone()).unwrap()],
            [Occupancy::calculate_presence(df2.clone()).unwrap()],
            [Occupancy::calculate_presence(df3.clone()).unwrap()],
            [Occupancy::calculate_presence(df4.clone()).unwrap()],
            [Occupancy::calculate_presence(df5.clone()).unwrap()],
        ];
        //let targets = array![1, 0];
        let targets = array![
            sigmoid.inverse(p(0.9)),
            sigmoid.inverse(p(0.1)),
            sigmoid.inverse(p(0.5)),
            sigmoid.inverse(p(0.6)),
            sigmoid.inverse(p(0.4)),
            sigmoid.inverse(p(0.7))
        ];

        let dataset = Dataset::new(features.clone(), targets.clone());

        println!("Features: {:?}", features);

        // Train logistic regression with L2 regularization
        let model = LinearRegression::default().fit(&dataset).expect("training failed");
        //let model = LogisticRegression::default().fit(&dataset).expect("training failed");

        let y_logit_pred = model.predict(&features);
        let p_pred = y_logit_pred.mapv(|z| 1.0 / (1.0 + (-z).exp()));
        println!("Predicted probabilities: {:?}", p_pred);
        println!("Expected probabilities: {:?}", targets.mapv(|z| 1.0 / (1.0 + (-z).exp())));

        println!("Coefficients: {:?}", model.params());
        println!("Prior: {:?}", model.intercept());

        let prior: f64 = model.intercept();
        let w_presence: f64 = model.params()[0];

        let cv0 = Occupancy::calculate(prior, w_presence, df0).unwrap();
        let cv1 = Occupancy::calculate(prior, w_presence, df1).unwrap();
        let cv2 = Occupancy::calculate(prior, w_presence, df2).unwrap();
        let cv3 = Occupancy::calculate(prior, w_presence, df3).unwrap();
        let cv4 = Occupancy::calculate(prior, w_presence, df4).unwrap();
        let cv5 = Occupancy::calculate(prior, w_presence, df5).unwrap();

        println!("cv0: {}", cv0.factor());
        println!("cv1: {}", cv1.factor());
        println!("cv2: {}", cv2.factor());
        println!("cv3: {}", cv3.factor());
        println!("cv4: {}", cv4.factor());
        println!("cv5: {}", cv5.factor());
    }
}
