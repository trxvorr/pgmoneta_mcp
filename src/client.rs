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

use super::configuration::CONFIG;
use super::constant::*;
use super::security::SecurityUtil;
use anyhow::anyhow;
use chrono::Local;
use serde::Serialize;
use std::fmt::Debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Represents the header of a request sent to the pgmoneta server.
///
/// Contains metadata such as the command code, client version,
/// formatting preferences, and security settings.
#[derive(Serialize, Clone, Debug)]
struct RequestHeader {
    #[serde(rename = "Command")]
    command: u32,
    #[serde(rename = "ClientVersion")]
    client_version: String,
    #[serde(rename = "Output")]
    output_format: u8,
    #[serde(rename = "Timestamp")]
    timestamp: String,
    #[serde(rename = "Compression")]
    compression: u8,
    #[serde(rename = "Encryption")]
    encryption: u8,
}

/// A wrapper structure that combines a request header with its specific payload.
///
/// This is the final serialized object sent over the TCP connection to pgmoneta.
#[derive(Serialize, Clone, Debug)]
struct PgmonetaRequest<R>
where
    R: Serialize + Clone + Debug,
{
    #[serde(rename = "Header")]
    header: RequestHeader,
    #[serde(rename = "Request")]
    request: R,
}

/// Handles network communication with the backend pgmoneta server.
///
/// This client manages the lifecycle of a request: building headers,
/// authenticating, opening a TCP stream, writing the payload, and reading the response.
pub struct PgmonetaClient;
impl PgmonetaClient {
    /// Constructs a standard request header for a given command.
    ///
    /// The header includes the current local timestamp and defaults to
    /// no encryption or compression, expecting a JSON response.
    fn build_request_header(command: u32) -> RequestHeader {
        let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
        RequestHeader {
            command,
            client_version: CLIENT_VERSION.to_string(),
            output_format: Format::JSON,
            timestamp,
            compression: Compression::NONE,
            encryption: Encryption::NONE,
        }
    }

    /// Establishes an authenticated TCP connection to the pgmoneta server.
    ///
    /// Looks up the provided `username` in the configuration to find the encrypted
    /// password, decrypts it using the master key, and initiates the connection.
    ///
    /// # Arguments
    /// * `username` - The admin username requesting the connection.
    ///
    /// # Returns
    /// An authenticated `TcpStream` ready for read/write operations.
    async fn connect_to_server(username: &str) -> anyhow::Result<TcpStream> {
        let config = CONFIG.get().expect("Configuration should be enabled");
        let security_util = SecurityUtil::new();

        if !config.admins.contains_key(username) {
            return Err(anyhow!(
                "request_backup_info: unable to find user {username}"
            ));
        }

        let password_encrypted = config
            .admins
            .get(username)
            .expect("Username should be found");
        let master_key = security_util.load_master_key()?;
        let password = String::from_utf8(
            security_util.decrypt_from_base64_string(password_encrypted, &master_key[..])?,
        )?;
        let stream = SecurityUtil::connect_to_server(
            &config.pgmoneta.host,
            config.pgmoneta.port,
            username,
            &password,
        )
        .await?;
        Ok(stream)
    }

    /// Writes a serialized JSON request string to the active TCP stream.
    ///
    /// Protocol flow:
    /// 1. Writes the compression flag.
    /// 2. Writes the encryption flag.
    /// 3. Writes the length of the payload.
    /// 4. Writes the exact payload bytes.
    async fn write_request(request_str: &str, stream: &mut TcpStream) -> anyhow::Result<()> {
        let mut request_buf = Vec::new();
        request_buf.write_i32(request_str.len() as i32).await?;
        request_buf.write_all(request_str.as_bytes()).await?;

        stream.write_u8(Compression::NONE).await?;
        stream.write_u8(Encryption::NONE).await?;
        stream.write_all(request_buf.as_slice()).await?;
        Ok(())
    }

    /// Reads the response payload from the TCP stream.
    ///
    /// Protocol flow:
    /// 1. Reads the compression flag.
    /// 2. Reads the encryption flag.
    /// 3. Reads the payload length.
    /// 4. Reads the exact number of bytes specified by the length.
    async fn read_response(stream: &mut TcpStream) -> anyhow::Result<String> {
        let _compression = stream.read_u8().await?;
        let _encryption = stream.read_u8().await?;
        let len = stream.read_u32().await? as usize;
        let mut response = vec![0u8; len];
        let n = stream.read_exact(&mut response).await?;
        let response_str = String::from_utf8(Vec::from(&response[..n]))?;
        Ok(response_str)
    }

    /// End-to-end wrapper for sending a request to the pgmoneta server and awaiting its response.
    ///
    /// # Arguments
    /// * `username` - The admin username making the request.
    /// * `command` - The numeric command code (e.g., `Command::INFO`).
    /// * `request` - The specific request payload object.
    ///
    /// # Returns
    /// The raw string response from the pgmoneta server.
    async fn forward_request<R>(username: &str, command: u32, request: R) -> anyhow::Result<String>
    where
        R: Serialize + Clone + Debug,
    {
        let mut stream = Self::connect_to_server(username).await?;
        tracing::info!(username = username, "Connected to server");

        let header = Self::build_request_header(command);
        let request = PgmonetaRequest { request, header };

        let request_str = serde_json::to_string(&request)?;
        Self::write_request(&request_str, &mut stream).await?;
        tracing::debug!(username = username, request = ?request, "Sent request to server");
        Self::read_response(&mut stream).await
    }
}
