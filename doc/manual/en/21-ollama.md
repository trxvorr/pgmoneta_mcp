\newpage

# Ollama

[Ollama](https://ollama.com) is the recommended way to get started with local LLM
support in **pgmoneta_mcp**. It provides a straightforward CLI, a local HTTP API,
and a simple model management workflow.

This chapter shows how to install Ollama, prepare a tool-capable model, and configure **pgmoneta_mcp** to use it.

## Install Ollama

Follow the installation instructions for your platform at
[ollama.com/download](https://ollama.com/download).

After installation, verify that the CLI is available:

``` sh
ollama --version
```

## Start the Ollama service

Ollama serves models over HTTP. Start it with:

``` sh
ollama serve
```

The default endpoint is:

``` text
http://localhost:11434
```

Check that it is reachable:

``` sh
curl http://localhost:11434/
```

If the service is up, the response should confirm that Ollama is running.

## Pull a model

Before using a model, download it locally. For example:

``` sh
ollama pull llama3.1
```

or:

``` sh
ollama pull qwen2.5:3b
```

You can inspect the locally available models with:

``` sh
ollama list
```

## Check tool support

For **pgmoneta_mcp**, the model must support tool calling.

To inspect a model:

``` sh
ollama show qwen2.5:3b
```

You can also query the API directly:

``` sh
curl -s http://localhost:11434/api/show \
  -d '{"model":"qwen2.5:3b"}'
```

Look for `tools` in the reported capabilities.

## Configure pgmoneta_mcp

Add or update the `[llm]` section in `pgmoneta-mcp.conf`:

``` ini
[llm]
provider = ollama
endpoint = http://localhost:11434
model = qwen2.5:3b
max_tool_rounds = 10
```

Property summary:

| Property | Description |
| :------- | :---------- |
| `provider` | Required. Must be `ollama` |
| `endpoint` | Required. Base URL of the Ollama server |
| `model` | Optional. Defaults to `llama3.1` for Ollama |
| `max_tool_rounds` | Safety limit for repeated tool-calling rounds |

If `model` is omitted, **pgmoneta_mcp** uses `llama3.1` as the default model name for Ollama.
