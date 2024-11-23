use super::StateChangedEvent;

pub trait GetAllEntityStatesPort {
    async fn get_current_state(&self) -> anyhow::Result<Vec<StateChangedEvent>>;
}

pub trait ListenToStateChangesPort {
    async fn recv(&mut self) -> anyhow::Result<StateChangedEvent>;
}

pub trait CallServicePort {
    async fn call_service(
        &self,
        domain: &str,
        service: &str,
        service_data: serde_json::Value,
    ) -> anyhow::Result<()>;
}
