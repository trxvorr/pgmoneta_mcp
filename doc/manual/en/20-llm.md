\newpage

# Local LLM

**pgmoneta_mcp** can be used together with a local large language model (LLM) so that
users can explore pgmoneta backups in natural language without sending prompts or
backup metadata to a cloud service.

The local LLM integration is built around three parts:

* The **pgmoneta MCP server**, which exposes pgmoneta operations as MCP tools
* A local **LLM runtime**, which hosts a tool-capable model
* The **agent** layer, which sends prompts to the model, executes requested tools,
  and returns a final answer

The overall flow looks like this:

``` text
User prompt
    |
    v
Local LLM runtime
    |
    v
pgmoneta_mcp tools
    |
    v
pgmoneta
```

## Current scope

In the current code base, the local LLM integration is implemented through the
optional `[llm]` configuration section and the `src/llm/ollama.rs` client.
That means the supported provider today is:

* `ollama`

## Local runtime landscape

The initial local runtime landscape for **pgmoneta_mcp** is:

| Runtime | Project | Status in pgmoneta_mcp | Notes |
| :------ | :------ | :--------------------- | :---- |
| Ollama | [ollama.com](https://ollama.com) | Supported and documented | Recommended starting point for most users |


## Requirements

To use a local model successfully with **pgmoneta_mcp**, make sure that:

* The runtime is reachable from the machine running the agent
* The selected model supports tool or function calling
* The model fits the available CPU, RAM, and optionally GPU resources
* The MCP server can already reach your pgmoneta management endpoint

Tool support matters because the model must be able to request calls such as
`list_backups` and `get_backup_info` instead of trying to invent backup data.

## Supported models

The following models are good starting points for local tool-calling workflows:

| Model | Size | RAM Needed | Notes |
| :---- | :--- | :--------- | :---- |
| `llama3.1:8b` | ~4.7 GB | ~8 GB | Balanced default choice |
| `llama3.2:3b` | ~2.0 GB | ~4 GB | Lightweight option |
| `qwen2.5:0.5b` | ~0.4 GB | ~1 GB | Minimal footprint |
| `qwen2.5:3b` | ~1.9 GB | ~4 GB | Good balance of speed and quality |
| `qwen2.5:7b` | ~4.7 GB | ~8 GB | Stronger tool-calling performance |
| `mistral:7b` | ~4.1 GB | ~8 GB | Solid general-purpose option |
| `mixtral:8x7b` | ~26.0 GB | ~32 GB | Higher quality with higher hardware cost |

## Configuration

Local LLM integration is configured through the optional `[llm]` section in
`pgmoneta-mcp.conf`.

``` ini
[llm]
provider = ollama
endpoint = http://localhost:11434
model = llama3.1
max_tool_rounds = 10
```

The available properties are:

| Property | Default | Required | Description |
| :------- | :------ | :------- | :---------- |
| `provider` |  | Yes | The local LLM backend |
| `endpoint` |  | Yes | Base URL for the local LLM server |
| `model` | `llama3.1` for Ollama | No | Model name sent to the runtime |
| `max_tool_rounds` | `10` | No | Maximum number of tool-calling rounds before aborting |

If you do not configure an `[llm]` section, the MCP server still works as a normal
tool server for MCP clients such as VS Code or other AI assistants.

## Next step

The next chapter documents the recommended local runtime, **Ollama**, including
installation, model setup, and configuration.
