use r#macro::{EnumVariants, Id};

use crate::{
    core::unit::RawValue,
    home::{
        LoadBalancedThermostat, Thermostat,
        action::{Rule, RuleEvaluationContext, RuleResult},
        command::Command,
    },
    home_state::RawVendorValue,
};

#[derive(Debug, Clone, PartialEq, Eq, Id, EnumVariants)]
pub enum ProvideLoadRoomMean {
    LivingRoom,
}

impl Rule for ProvideLoadRoomMean {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let ((factor1, thermostat1), (factor2, thermostat2)) = match self {
            ProvideLoadRoomMean::LivingRoom => (
                (
                    Thermostat::LivingRoomBig.heating_factor(),
                    LoadBalancedThermostat::LivingRoomBig,
                ),
                (
                    Thermostat::LivingRoomSmall.heating_factor(),
                    LoadBalancedThermostat::LivingRoomSmall,
                ),
            ),
        };

        let load1 = ctx.current(RawVendorValue::AllyLoadEstimate(Thermostat::from(&thermostat1)))?;
        let load2 = ctx.current(RawVendorValue::AllyLoadEstimate(Thermostat::from(&thermostat2)))?;

        if !is_valid(load1) || !is_valid(load2) {
            return Ok(RuleResult::Skip);
        }

        let mean = ((factor1 * load1.0 + factor2 * load2.0) / (factor1 + factor2)).round();

        Ok(RuleResult::Execute(vec![
            Command::SetThermostatLoadMean {
                device: thermostat1.clone(),
                value: RawValue(mean),
            },
            Command::SetThermostatLoadMean {
                device: thermostat2.clone(),
                value: RawValue(mean),
            },
        ]))
    }
}

fn is_valid(value: RawValue) -> bool {
    value.0 > -500.0 && value.0 <= 3600.0
}
