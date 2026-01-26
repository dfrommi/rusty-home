mod degree_celsius;
mod density;
mod derivative;
mod fan;
mod heating;
mod kwh;
mod light;
mod liquid;
mod percent;
mod probability;
mod watt;

pub use degree_celsius::DegreeCelsius;
pub use density::GramPerCubicMeter;
pub use derivative::RateOfChange;
pub use fan::*;
pub use heating::HeatingUnit;
pub use kwh::KiloWattHours;
pub use light::Lux;
pub use liquid::KiloCubicMeter;
pub use percent::Percent;
pub use probability::Probability;
pub use probability::p;
pub use watt::Watt;

macro_rules! v {
    ($x:literal C) => {
        DegreeCelsius($x as f64)
    };
    ($x:literal %) => {
        Percent($x as f64)
    };
}

const C: DegreeCelsius = DegreeCelsius(1.0);
const PCT: Percent = Percent(1.0);

impl std::ops::Mul<DegreeCelsius> for usize {
    type Output = DegreeCelsius;

    fn mul(self, rhs: DegreeCelsius) -> Self::Output {
        DegreeCelsius(self as f64 * rhs.0)
    }
}

impl std::ops::Mul<Percent> for usize {
    type Output = Percent;

    fn mul(self, rhs: Percent) -> Self::Output {
        Percent(self as f64 * rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degree_celsius() {
        assert_eq!(v!(25 C), DegreeCelsius(25.0));
        assert_eq!(25.0 * C, DegreeCelsius(25.0));
        assert_eq!(25 * C, DegreeCelsius(25.0));
    }

    #[test]
    fn test_percent() {
        assert_eq!(v!(50 %), Percent(50.0));
        assert_eq!(50.0 * PCT, Percent(50.0));
        assert_eq!(50 * PCT, Percent(50.0));
    }
}
