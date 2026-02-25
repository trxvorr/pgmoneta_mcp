// Copyright (C) 2026 The pgmoneta community
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

mod info;

use super::client::PgmonetaClient;
use super::constant::*;
use super::constant::{Command, Compression, Encryption};
use crate::utils::Utility;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde_json::Map;
use serde_json::Value;

/// The core handler for incoming Model Context Protocol (MCP) requests.
///
/// This struct routes MCP tool calls from the client (like an AI model)
/// to the appropriate internal functions that communicate with pgmoneta.
#[derive(Clone)]
pub struct PgmonetaHandler {
    tool_router: ToolRouter<PgmonetaHandler>,
}

#[tool_router]
impl PgmonetaHandler {
    /// Creates a new instance of the `PgmonetaHandler` with an initialized tool router.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Simple ping tool to verify the MCP server is responsive.
    #[tool(description = "Say hello to the client")]
    fn say_hello(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "Hello from pgmoneta MCP server!",
        )]))
    }

    /// Tool for fetching detailed information about a specific backup.
    #[tool(
        description = "Get information of a backup using given backup ID and server name. \
    \"newest\", \"latest\" or \"oldest\" are also accepted as backup identifier.\
    The username has to be one of the pgmoneta admins to be able to access pgmoneta"
    )]
    async fn get_backup_info(
        &self,
        Parameters(args): Parameters<info::InfoRequest>,
    ) -> Result<CallToolResult, McpError> {
        let result = self._get_backup_info(args).await?;
        Self::_generate_call_tool_result(&result)
    }

    /// Tool for listing available backups on a specified server.
    #[tool(description = "List backups of a server. \
        Specify asc or desc to determine the sorting order.\
        The backups are sorted in ascending order if not specified.")]
    async fn list_backups(
        &self,
        Parameters(args): Parameters<info::ListBackupsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let result = self._list_backups(args).await?;
        Self::_generate_call_tool_result(&result)
    }
}

impl PgmonetaHandler {
    /// Parses the raw string response from pgmoneta into a JSON map.
    ///
    /// Ensures the response contains the required `Outcome` category key.
    fn _parse_and_check_result(result: &str) -> Result<Map<String, Value>, McpError> {
        let response: Map<String, Value> = serde_json::from_str(result).map_err(|e| {
            McpError::parse_error(format!("Failed to parse result {result}: {:?}", e), None)
        })?;
        if !response.contains_key(MANAGEMENT_CATEGORY_OUTCOME) {
            return Err(McpError::internal_error(
                format!("Fail to find outcome inside response {:?}", response),
                None,
            ));
        }
        Ok(response)
    }

    /// Recursively translates numeric/raw fields in the pgmoneta response into human-readable formats.
    ///
    /// This includes:
    /// * Formatting byte counts into human-readable file sizes (e.g., KB, MB).
    /// * Converting LSNs (Log Sequence Numbers) into hex strings.
    /// * Translating numeric enum codes (Compression, Encryption, Error) into descriptive strings.
    fn _translate_result<'a, M>(map: M) -> anyhow::Result<Map<String, Value>>
    where
        M: IntoIterator<Item = (&'a String, &'a Value)>,
    {
        // fields to be translated
        // file size, hex string, compression, encryption, command method, object(recursive), error
        let file_size_fields = vec![
            "BackupSize",
            "RestoreSize",
            "BiggestFileSize",
            "Delta",
            "TotalSpace",
            "FreeSpace",
            "UsedSpace",
            "WorkspaceFreeSpace",
            "HotStandbySize",
        ];
        let hex_string_fields = [
            "CheckpointHiLSN",
            "CheckpointLoLSN",
            "StartHiLSN",
            "StartLoLSN",
            "EndHiLSN",
            "EndLoLSN",
        ];
        let object_arr_fields = ["Backups"];
        let compression_field = "Compression";
        let encryption_field = "Encryption";
        let command_field = "Command";
        let error_field = "Error";

        let mut trans_res: Map<String, Value> = Map::new();
        for (key, value) in map {
            if file_size_fields.contains(&key.as_str()) {
                let size = value.as_u64().unwrap();
                let size_str = Utility::format_file_size(size);
                trans_res.insert(key.clone(), Value::from(size_str));
            } else if hex_string_fields.contains(&key.as_str()) {
                let num = value.as_u64().unwrap();
                let hex_str = format!("0x{:X}", num);
                trans_res.insert(key.clone(), Value::from(hex_str));
            } else if key == compression_field {
                let compression = value.as_u64().unwrap();
                let compression_str = Compression::translate_compression_enum(compression as u8)?;
                trans_res.insert(key.clone(), Value::from(compression_str));
            } else if key == encryption_field {
                let encryption = value.as_u64().unwrap();
                let encryption_str = Encryption::translate_encryption_enum(encryption as u8)?;
                trans_res.insert(key.clone(), Value::from(encryption_str));
            } else if key == command_field {
                let command = value.as_u64().unwrap();
                let command_str = Command::translate_command_enum(command as u32)?;
                trans_res.insert(key.clone(), Value::from(command_str));
            } else if key == error_field {
                let error = value.as_u64().unwrap();
                let error_msg = ManagementError::translate_error_enum(error as u32);
                trans_res.insert(key.clone(), Value::from(error_msg));
            } else if object_arr_fields.contains(&key.as_str()) {
                let mut trans_arr: Vec<Value> = Vec::new();
                if value.as_array().is_none() {
                    trans_res.insert(key.clone(), Value::from(trans_arr));
                    return Ok(trans_res);
                }
                let arr = value.as_array().unwrap();
                for item in arr {
                    if let Value::Object(object) = item {
                        let trans_obj = Self::_translate_result(object)?;
                        trans_arr.push(Value::Object(trans_obj));
                    } else {
                        trans_arr.push(item.clone())
                    }
                }
                trans_res.insert(key.clone(), Value::from(trans_arr));
                return Ok(trans_res);
            } else if value.is_object() {
                let object = value.as_object().unwrap();
                let trans_obj = Self::_translate_result(object)?;
                trans_res.insert(key.clone(), Value::Object(trans_obj));
            } else {
                trans_res.insert(key.clone(), value.clone());
            }
        }
        Ok(trans_res)
    }

    /// Wraps the parsed and translated pgmoneta response into a standardized `CallToolResult`
    /// that can be sent back to the MCP client.
    fn _generate_call_tool_result(result: &str) -> Result<CallToolResult, McpError> {
        let res = Self::_parse_and_check_result(result)?;
        let trans_res = Self::_translate_result(&res).map_err(|e| {
            McpError::internal_error(
                format!("Failed to translate some of the result fields: {:?}", e),
                None,
            )
        })?;
        let trans_res_str = serde_json::to_string(&trans_res).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {:?}", e), None)
        })?;
        Ok(CallToolResult::success(vec![Content::text(trans_res_str)]))
    }
}

impl Default for PgmonetaHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for PgmonetaHandler {
    /// Provides the MCP initialization capabilities and metadata for this server.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides capabilities to interact with pgmoneta, a backup/restore tool for PostgreSQL.".to_string()),
        }
    }

    /// Handles the initial connection setup and handshake from an MCP client.
    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}
