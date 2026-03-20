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

use super::{ChatMessage, LlmClient, LlmResponse, ToolCall, ToolCallFunction, ToolDefinition};
use anyhow::anyhow;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Client for communicating with an Ollama LLM inference server.
///
/// Uses Ollama's native `/api/chat` endpoint which supports tool/function calling.
/// Ollama must be running locally (or at the configured endpoint) with a model
/// that supports tool calling (e.g., `llama3.1`, `qwen2.5`).
pub struct OllamaClient {
    http_client: Client,
    endpoint: String,
    model: String,
}

/// Request body for Ollama's `/api/chat` endpoint.
#[derive(Debug, Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [ToolDefinition]>,
}

/// Response from Ollama's `/api/chat` endpoint.
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    #[allow(dead_code)]
    done: bool,
}

/// A message within Ollama's chat response.
#[derive(Debug, Deserialize)]
struct OllamaMessage {
    #[allow(dead_code)]
    role: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<OllamaToolCall>,
}

/// A tool call in Ollama's response format.
#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    function: OllamaToolCallFunction,
}

/// Function details within an Ollama tool call.
#[derive(Debug, Deserialize)]
struct OllamaToolCallFunction {
    name: String,
    arguments: HashMap<String, Value>,
}

/// Model information returned by Ollama's `/api/tags` endpoint.
#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelInfo>,
}

/// Individual model details from the tags endpoint.
#[derive(Debug, Deserialize)]
pub struct OllamaModelInfo {
    /// The model name (e.g., `llama3.1:latest`).
    pub name: String,
    /// Details about the model.
    pub details: Option<OllamaModelDetails>,
}

/// Model detail fields from Ollama.
#[derive(Debug, Deserialize)]
pub struct OllamaModelDetails {
    /// The model family (e.g., `llama`, `qwen2`).
    pub family: Option<String>,
    /// The parameter size (e.g., `8.0B`).
    pub parameter_size: Option<String>,
    /// The quantization level (e.g., `Q4_K_M`).
    pub quantization_level: Option<String>,
}

/// Response from Ollama's `/api/show` endpoint.
#[derive(Debug, Deserialize)]
struct OllamaShowResponse {
    #[serde(default)]
    capabilities: Vec<String>,
}

impl OllamaClient {
    /// Creates a new `OllamaClient`.
    ///
    /// # Arguments
    /// * `endpoint` - The base URL of the Ollama server (e.g., `http://localhost:11434`).
    /// * `model` - The model name to use for inference (e.g., `llama3.1`).
    pub fn new(endpoint: &str, model: &str) -> Self {
        Self {
            http_client: Client::new(),
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Checks whether the Ollama server is running and reachable.
    ///
    /// # Returns
    /// `Ok(())` if the server responds, or an error describing the connection failure.
    pub async fn health_check(&self) -> anyhow::Result<()> {
        let url = format!("{}/", self.endpoint);
        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Cannot reach Ollama at {}: {}", self.endpoint, e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "Ollama health check failed with status {}",
                resp.status()
            ));
        }
        Ok(())
    }

    /// Checks whether the configured model is available locally and supports tool calling.
    ///
    /// # Returns
    /// `Ok(true)` if the model supports tools, `Ok(false)` if it does not,
    /// or an error if the model is not found.
    pub async fn check_model_capabilities(&self) -> anyhow::Result<bool> {
        let url = format!("{}/api/show", self.endpoint);
        let body = serde_json::json!({ "model": self.model });
        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to query model info: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "Model '{}' not found. Pull it first with: ollama pull {}",
                self.model,
                self.model
            ));
        }

        let show_resp: OllamaShowResponse = resp.json().await?;
        Ok(show_resp.capabilities.iter().any(|c| c == "tools"))
    }

    /// Lists all models available locally on the Ollama server.
    ///
    /// # Returns
    /// A vector of [`OllamaModelInfo`] describing each available model.
    pub async fn list_models(&self) -> anyhow::Result<Vec<OllamaModelInfo>> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to list models: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to list models, status: {}", resp.status()));
        }

        let tags: OllamaTagsResponse = resp.json().await?;
        Ok(tags.models)
    }

    /// Returns the model name this client is configured to use.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns the endpoint URL this client connects to.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl LlmClient for OllamaClient {
    /// Sends a chat request to the Ollama server with conversation history and tool definitions.
    ///
    /// Returns either a text response or a list of tool calls the model wants to invoke.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> anyhow::Result<LlmResponse> {
        let url = format!("{}/api/chat", self.endpoint);

        let request = OllamaChatRequest {
            model: &self.model,
            messages,
            stream: false,
            tools: if tools.is_empty() { None } else { Some(tools) },
        };

        tracing::debug!(model = %self.model, messages = messages.len(), tools = tools.len(), "Sending chat request to Ollama");

        let resp = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send chat request to Ollama: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Ollama chat request failed (status {}): {}",
                status,
                body
            ));
        }

        let chat_resp: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Ollama chat response: {}", e))?;

        if !chat_resp.message.tool_calls.is_empty() {
            let tool_calls = chat_resp
                .message
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    function: ToolCallFunction {
                        name: tc.function.name,
                        arguments: tc.function.arguments,
                    },
                })
                .collect();
            Ok(LlmResponse::ToolCalls(tool_calls))
        } else {
            Ok(LlmResponse::Text(chat_resp.message.content))
        }
    }
}
