mod command;
mod state;

pub use command::process_commands;
pub use state::export_state;

struct MqttStateValue(String);

impl From<bool> for MqttStateValue {
    fn from(val: bool) -> Self {
        MqttStateValue(if val {
            "1".to_string()
        } else {
            "0".to_string()
        })
    }
}

impl TryInto<bool> for MqttStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self.0.as_str() {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => anyhow::bail!("Error converting {} to bool", self.0),
        }
    }
}
