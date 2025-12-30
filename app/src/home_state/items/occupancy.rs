use crate::core::{
    math::Sigmoid,
    timeseries::{DataFrame, interpolate::LastSeenInterpolator},
};

use super::*;
use anyhow::Result;

use crate::core::math::DataFrameStatsExt as _;
use crate::core::time::DateTimeRange;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Occupancy {
    LivingRoomCouch,
    BedroomBed,
    RoomOfRequirementsDesk,
}

pub struct OccupancyStateProvider;

impl DerivedStateProvider<Occupancy, Probability> for OccupancyStateProvider {
    fn calculate_current(&self, id: Occupancy, ctx: &StateCalculationContext) -> Option<Probability> {
        let range = DateTimeRange::since(t!(1 hours ago));

        let main_df = match id {
            Occupancy::LivingRoomCouch => ctx.all_since(Presence::LivingRoomCouch, *range.start())?,
            Occupancy::BedroomBed => ctx.all_since(Presence::BedroomBed, *range.start())?,
            Occupancy::RoomOfRequirementsDesk => ctx.all_since(IsRunning::RoomOfRequirementsMonitor, *range.start())?,
        };

        let prior: f64 = -1.7968470630447446;
        let w_presence: f64 = 3.733237448369802;

        Occupancy::calculate(prior, w_presence, main_df)
    }
}

impl Occupancy {
    pub fn calculate_presence(presence: DataFrame<bool>) -> Option<f64> {
        let presence = presence.retain_range(
            &DateTimeRange::since(t!(1 hours ago)),
            LastSeenInterpolator,
            LastSeenInterpolator,
        );

        if presence.is_empty() {
            return None;
        }

        let s1 = presence.weighted_aged_sum(t!(30 minutes), LastSeenInterpolator);
        Some(s1)
    }

    pub fn calculate(prior: f64, w_presence: f64, presence: DataFrame<bool>) -> Option<Probability> {
        let sigmoid = Sigmoid::default();
        let s1 = Self::calculate_presence(presence)?;

        //let prior = logit(p(0.03));

        Some(sigmoid.eval(prior + w_presence * s1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfa::prelude::*;
    use linfa_linear::LinearRegression;
    use ndarray::array;

    use crate::core::{
        math::Sigmoid,
        timeseries::{DataFrame, DataPoint},
    };

    #[test]
    fn training() {
        let sigmoid = Sigmoid::default();

        let df0 = DataFrame::new(vec![DataPoint::new(true, t!(1 hours ago))]);
        let df1 = DataFrame::new(vec![DataPoint::new(false, t!(1 hours ago))]);
        let df2 = DataFrame::new(vec![
            DataPoint::new(false, t!(1 hours ago)),
            DataPoint::new(true, t!(15 minutes ago)),
            DataPoint::new(false, t!(2 minutes ago)),
            DataPoint::new(true, t!(1 minutes ago)),
        ]);

        let df3 = DataFrame::new(vec![
            DataPoint::new(true, t!(1 hours ago)),
            DataPoint::new(false, t!(20 minutes ago)),
            DataPoint::new(true, t!(1 minutes ago)),
        ]);
        let df4 = DataFrame::new(vec![
            DataPoint::new(false, t!(1 hours ago)),
            DataPoint::new(true, t!(10 minutes ago)),
        ]);
        let df5 = DataFrame::new(vec![
            DataPoint::new(true, t!(1 hours ago)),
            DataPoint::new(false, t!(5 minutes ago)),
            DataPoint::new(true, t!(4 minutes ago)),
            DataPoint::new(false, t!(3 minutes ago)),
            DataPoint::new(true, t!(2 minutes ago)),
            DataPoint::new(false, t!(1 minutes ago)),
        ]);

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
