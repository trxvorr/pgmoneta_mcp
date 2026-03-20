# Local LLM Installation

This guide focuses on installing and configuring a local LLM runtime for pgmoneta MCP.
It is intentionally scoped to local setup and does not cover chat examples or internal
implementation details.

## Scope

This guide covers:

* Installing Ollama
* Downloading and validating a model
* Configuring the `[llm]` section in `pgmoneta-mcp.conf`

## Ollama

[Ollama](https://ollama.com) is the recommended provider for running open-source models locally.
It provides a simple CLI and API for downloading, managing, and serving LLM models.

### Install

To install Ollama, follow instructions at [ollama.com/download](https://ollama.com/download).

### Verify installation

```
ollama --version
```

### Start the Ollama server

Ollama runs as a background service. Start it with

```
ollama serve
```

On Linux with systemd, it may already be running as a service after installation.

Verify it is running

```
curl http://localhost:11434/
```

This should print `Ollama is running`.

### Download models

Models must be downloaded before they can be used. This is the only step that requires
network access. Once downloaded, models are cached locally and work fully offline.

* Pull a model

    ```
    ollama pull llama3.1
    ```

* List downloaded models

    ```
    ollama list
    ```

* Test a model

    ```
    ollama run llama3.1 "Hello, what can you do?"
    ```

### Recommended models

The model must support **tool calling** (function calling) to work with pgmoneta MCP tools.

| Model | Size | RAM Needed | Tool Calling | Notes |
| :---- | :--- | :--------- | :----------- | :---- |
| `llama3.1:8b` | ~4.7 GB | ~8 GB | Yes | **Default**. Best balance of capability and size |
| `llama3.2:3b` | ~2.0 GB | ~4 GB | Yes | Lightweight option for limited hardware |
| `qwen2.5:0.5b` | ~0.4 GB | ~1 GB | Yes | Extremely lightweight |
| `qwen2.5:3b` | ~1.9 GB | ~4 GB | Yes | Great balance of speed and capabilities |
| `qwen2.5:7b` | ~4.7 GB | ~8 GB | Yes | Excellent tool calling capabilities |
| `mistral:7b` | ~4.1 GB | ~8 GB | Yes | Strong performance for open-source models |
| `mixtral:8x7b` | ~26.0 GB | ~32 GB | Yes | High quality MoE model |

### Check tool support

You can verify that a model supports tool calling

```
ollama show llama3.1
```

Look for `tools` in the capabilities list. Alternatively, query the API

```
curl -s http://localhost:11434/api/show -d '{"model": "llama3.1"}' | grep capabilities
```

### Configuration

Add an `[llm]` section to your `pgmoneta-mcp.conf`

```
[llm]
provider = ollama
endpoint = http://localhost:11434
model = llama3.1
max_tool_rounds = 10
```

### Configuration properties

| Property | Default | Required | Description |
| :------- | :------ | :------- | :---------- |
| provider |  | Yes | The LLM provider backend |
| endpoint |  | Yes | The URL of the LLM inference server |
| model | llama3.1 (Ollama) | No | The model name to use for inference |
| max_tool_rounds | 10 | No | Maximum tool-calling iterations per user prompt |

### Quick verification

1. Confirm Ollama is running:

```
curl http://localhost:11434/
```

2. Start pgmoneta MCP with your config file:

```
pgmoneta-mcp-server -c pgmoneta-mcp.conf -u pgmoneta-mcp-users.conf
```

3. From your MCP client, ask for backup information to verify end-to-end setup.
