use crate::core::HomeApi;
use crate::core::id::ExternalId;
use crate::core::time::{DateTime, Duration};
use crate::home::state::HomeState;
use crate::port::DataPointAccess;
use rmcp::handler::server::tool::Parameters;
use rmcp::{
    RoleServer, ServerHandler, handler::server::router::tool::ToolRouter, model::ErrorData as McpError, model::*,
    service::RequestContext, tool_handler, tool_router,
};
use rmcp::{schemars, tool};

#[derive(Clone)]
pub struct SmartHomeMcp {
    api: HomeApi,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeviceIds {
    pub devices: Vec<DeviceId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct DeviceId {
    pub device_type: String,
    pub device_name: String,
}

impl From<DeviceId> for ExternalId {
    fn from(val: DeviceId) -> Self {
        ExternalId::new(val.device_type, val.device_name)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DeviceState {
    device: DeviceId,
    value: String,
    last_changed: DateTime,
    same_value_duration: Duration,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DeviceError {
    device: DeviceId,
    error: String,
}

#[tool_router]
impl SmartHomeMcp {
    pub fn new(api: HomeApi) -> Self {
        Self {
            api,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get current date and time")]
    fn get_now(&self) -> Result<CallToolResult, McpError> {
        let now = DateTime::now();
        Ok(CallToolResult::success(vec![Content::json(now)?]))
    }

    #[tool(description = "List all available smart home devices with their types and names")]
    async fn list_devices(&self) -> Result<CallToolResult, McpError> {
        let mut devices = Vec::new();

        for state in HomeState::variants() {
            let external_id: &ExternalId = state.as_ref();

            devices.push(Content::json(DeviceId {
                device_type: external_id.ext_type().to_string(),
                device_name: external_id.ext_name().to_string(),
            })?);
        }

        Ok(CallToolResult::success(devices))
    }

    #[tool(description = "Get the current state of multiple smart home devices at once")]
    async fn get_device_states(
        &self,
        Parameters(DeviceIds { devices }): Parameters<DeviceIds>,
    ) -> Result<CallToolResult, McpError> {
        let mut results = Vec::new();

        for device_id in devices {
            let ext_id: ExternalId = device_id.clone().into();

            match HomeState::try_from(ext_id) {
                Ok(state) => match state.current_data_point(&self.api).await {
                    Ok(data_point) => {
                        results.push(Content::json(DeviceState {
                            device: device_id.clone(),
                            value: data_point.value.value_to_string(),
                            last_changed: data_point.timestamp,
                            same_value_duration: data_point.timestamp.elapsed(),
                        })?);
                    }

                    Err(e) => {
                        results.push(Content::json(DeviceError {
                            device: device_id.clone(),
                            error: format!("Failed to get state value: {e}"),
                        })?);
                    }
                },

                Err(_) => {
                    results.push(Content::json(DeviceError {
                        device: device_id.clone(),
                        error: format!("Unknown device: {}/{}", device_id.device_type, device_id.device_name),
                    })?);
                }
            }
        }

        Ok(CallToolResult::success(results))
    }
}

#[tool_handler]
impl ServerHandler for SmartHomeMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides access to smart home device states. \
                 You can list all available devices and read their current values. \
                 Use the list_devices tool to discover devices, then \
                 get_device_states tool with device type and name."
                    .to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _request: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::method_not_found::<ReadResourceRequestMethod>())
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _request: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Err(McpError::method_not_found::<GetPromptRequestMethod>())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}
