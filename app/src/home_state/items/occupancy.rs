use crate::core::{
    math::Sigmoid,
    time::Duration,
    timeseries::{DataFrame, interpolate::LastSeenInterpolator},
};

use super::*;

use crate::core::math::DataFrameStatsExt as _;
use crate::core::time::DateTimeRange;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Occupancy {
    LivingRoomCouch,
    LivingRoomCouchShort,
    BedroomBed,
    RoomOfRequirementsDesk,
}

pub struct OccupancyStateProvider;

impl DerivedStateProvider<Occupancy, Probability> for OccupancyStateProvider {
    fn calculate_current(&self, id: Occupancy, ctx: &StateCalculationContext) -> Option<Probability> {
        let since = t!(1 hours ago);
        match id {
            Occupancy::LivingRoomCouch => {
                let df = ctx.all_since(Presence::LivingRoomCouch, since)?;
                Occupancy::calculate(-1.7968470630447446, 8.635109947226839, t!(30 minutes), df)
            }
            Occupancy::LivingRoomCouchShort => {
                let df = ctx.all_since(Presence::LivingRoomCouch, since)?;
                Occupancy::calculate(-2.5507124246455235, 97.5596255787969, t!(4 minutes), df)
            }
            Occupancy::BedroomBed => {
                let df = ctx.all_since(Presence::BedroomBed, since)?;
                Occupancy::calculate(-1.7968470630447446, 8.635109947226839, t!(30 minutes), df)
            }
            Occupancy::RoomOfRequirementsDesk => {
                let df = ctx.all_since(IsRunning::RoomOfRequirementsMonitor, since)?;
                Occupancy::calculate(-1.7968470630447446, 8.635109947226839, t!(30 minutes), df)
            }
        }
    }
}

impl Occupancy {
    pub fn calculate_presence(presence: DataFrame<bool>, tau: Duration) -> Option<f64> {
        let presence = presence.retain_range(
            &DateTimeRange::since(t!(1 hours ago)),
            LastSeenInterpolator,
            LastSeenInterpolator,
        );

        if presence.is_empty() {
            return None;
        }

        let s1 = presence.weighted_aged_sum(tau, LastSeenInterpolator);
        Some(s1)
    }

    pub fn calculate(prior: f64, w_presence: f64, tau: Duration, presence: DataFrame<bool>) -> Option<Probability> {
        let sigmoid = Sigmoid::default();
        let s1 = Self::calculate_presence(presence, tau)?;

        Some(sigmoid.eval(prior + w_presence * s1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfa::prelude::*;
    use linfa_linear::LinearRegression;
    use ndarray::{Array1, Array2};

    use crate::core::{
        math::Sigmoid,
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
    };

    fn df(points: &[(bool, DateTime)]) -> DataFrame<bool> {
        DataFrame::new(points.iter().map(|(v, ts)| DataPoint::new(*v, *ts)))
    }

    fn train_presence_model(samples: &[(DataFrame<bool>, Probability)], tau: Duration) -> (f64, f64) {
        let sigmoid = Sigmoid::default();
        let features = samples
            .iter()
            .map(|(df, _)| Occupancy::calculate_presence(df.clone(), tau.clone()).unwrap())
            .collect::<Vec<_>>();
        let targets = samples
            .iter()
            .map(|(_, probability)| sigmoid.inverse(*probability))
            .collect::<Vec<_>>();

        let features = Array2::from_shape_vec((samples.len(), 1), features).unwrap();
        let targets = Array1::from_vec(targets);
        let dataset = Dataset::new(features, targets);
        let model = LinearRegression::default().fit(&dataset).expect("training failed");

        (model.intercept(), model.params()[0])
    }

    #[test]
    fn training() {
        let df0 = df(&[(true, t!(1 hours ago))]);
        let df1 = df(&[(false, t!(1 hours ago))]);
        let df2 = df(&[
            (false, t!(1 hours ago)),
            (true, t!(15 minutes ago)),
            (false, t!(2 minutes ago)),
            (true, t!(1 minutes ago)),
        ]);
        let df3 = df(&[
            (true, t!(1 hours ago)),
            (false, t!(20 minutes ago)),
            (true, t!(1 minutes ago)),
        ]);
        let df4 = df(&[(false, t!(1 hours ago)), (true, t!(10 minutes ago))]);
        let df5 = df(&[
            (true, t!(1 hours ago)),
            (false, t!(5 minutes ago)),
            (true, t!(4 minutes ago)),
            (false, t!(3 minutes ago)),
            (true, t!(2 minutes ago)),
            (false, t!(1 minutes ago)),
        ]);

        let samples = [
            (df0.clone(), p(0.9)),
            (df1.clone(), p(0.1)),
            (df2.clone(), p(0.5)),
            (df3.clone(), p(0.6)),
            (df4.clone(), p(0.4)),
            (df5.clone(), p(0.7)),
        ];
        let (prior, w_presence) = train_presence_model(&samples, t!(30 minutes));

        println!("Prior: {:?}", prior);
        println!("w_presence: {:?}", w_presence);

        let cv0 = Occupancy::calculate(prior, w_presence, t!(30 minutes), df0).unwrap();
        let cv1 = Occupancy::calculate(prior, w_presence, t!(30 minutes), df1).unwrap();
        let cv2 = Occupancy::calculate(prior, w_presence, t!(30 minutes), df2).unwrap();
        let cv3 = Occupancy::calculate(prior, w_presence, t!(30 minutes), df3).unwrap();
        let cv4 = Occupancy::calculate(prior, w_presence, t!(30 minutes), df4).unwrap();
        let cv5 = Occupancy::calculate(prior, w_presence, t!(30 minutes), df5).unwrap();

        println!("cv0: {}", cv0.factor());
        println!("cv1: {}", cv1.factor());
        println!("cv2: {}", cv2.factor());
        println!("cv3: {}", cv3.factor());
        println!("cv4: {}", cv4.factor());
        println!("cv5: {}", cv5.factor());
    }

    #[test]
    fn training_short() {
        // Goal: ~90% density over 3 min → ~0.8, ~20% density → ~0.2
        // Small tau (4 min) forgets history quickly, so only recent density matters.
        // Feature ordering verified monotone before assigning labels.
        let tau = t!(4 minutes);

        // 1. Always present: maximum recent density
        let df_full = df(&[(true, t!(1 hours ago))]);
        // 2. Present for last 10 min: nearly identical to always-present at this tau
        let df_10min = df(&[(false, t!(1 hours ago)), (true, t!(10 minutes ago))]);
        // 3. Present for last 3 min solid: strong recent signal, anchor → ~0.8
        let df_3min = df(&[(false, t!(1 hours ago)), (true, t!(3 minutes ago))]);
        // 4. ~50% duty: alternating 1-min cycles over 6 min, currently true
        let df_half = df(&[
            (false, t!(6 minutes ago)),
            (true, t!(5 minutes ago)),
            (false, t!(4 minutes ago)),
            (true, t!(3 minutes ago)),
            (false, t!(2 minutes ago)),
            (true, t!(1 minutes ago)),
        ]);
        // 5. Low density: 30 seconds of presence, 90–120 seconds ago, anchor → ~0.2
        let df_sparse = df(&[
            (false, t!(10 minutes ago)),
            (true, t!(2 minutes ago)),
            (false, t!(90 seconds ago)),
        ]);
        // 6. Off for 3 minutes
        let df_off_3min = df(&[(true, t!(1 hours ago)), (false, t!(3 minutes ago))]);
        // 7. Always absent: zero recent presence
        let df_never = df(&[(false, t!(1 hours ago))]);

        let samples = [
            (df_full.clone(), p(0.99)),     // saturated ceiling — extended presence
            (df_10min.clone(), p(0.98)),    // nearly identical feature to full at this tau
            (df_3min.clone(), p(0.80)),     // anchor: solid 3 min → 0.8
            (df_half.clone(), p(0.70)),     // ~50% density is meaningfully lower
            (df_sparse.clone(), p(0.20)),   // anchor: ~20% density → 0.2
            (df_off_3min.clone(), p(0.05)), // strong recent absence signal, but some history of presence
            (df_never.clone(), p(0.13)),    // background: no recent presence
        ];
        let (prior, w_presence) = train_presence_model(&samples, tau.clone());

        println!("Prior: {prior}");
        println!("w_presence: {w_presence}");

        for (name, sample_df) in [
            ("always present", df_full),
            ("present 10min ", df_10min),
            ("present 3min  ", df_3min),
            ("~50% duty     ", df_half),
            ("sparse ~20%   ", df_sparse),
            ("off for 3min  ", df_off_3min),
            ("always absent ", df_never),
        ] {
            let v = Occupancy::calculate(prior, w_presence, tau.clone(), sample_df).unwrap();
            println!("{name}: {:.3}", v.factor());
        }
    }
}
