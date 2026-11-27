# Kaggle Studio MCP

This optional local STDIO server reuses the app's model catalog, pinned inference
runner and private Kaggle CLI backend. No third-party Kaggle plugin was found in
the connected plugin catalog during this implementation; this is an app-owned
adapter over the official SDK, not a claimed external connection.

Install `python3 scripts/setup_generation.py`, configure Connections in the app,
and create a destination folder. An MCP-capable agent can use:

```json
{
  "mcpServers": {
    "canvas-kaggle": {
      "command": "python3",
      "args": ["/absolute/path/to/canvas-studio/agents/kaggle_studio_mcp.py", "--workspace", "/absolute/path/to/AI-outputs"]
    }
  }
}
```

Tools:

- `kaggle_model_catalog`: reviewed recipes and connection booleans; no secrets.
- `kaggle_model_search`: live HF/Civitai metadata, no preview-image downloads.
- `kaggle_prepare_inference`: offline private notebook ZIP in the output workspace.
- `kaggle_submit_inference`: finite private T4 job using available Kaggle quota.
- `kaggle_inference_status`: inspect a previously recorded document-owned run.
- `kaggle_download_output`: checksum-verified output into the output workspace.

Each call uses an explicit `document_id` to scope remote runs. Model IDs are from
the reviewed catalog; settings are typed and bounded. There are no credential,
shell, arbitrary-code or paid-inference tools. Notebook ZIPs include prompts and
explicit references but not tokens. Configure gated-download secrets within
Kaggle; never give credentials to a coding agent in chat.

The existing live-editor MCP now has **15 tools**. Its `studio_generation` tool
adds discovery/registration, notebook preparation, local inference and Kaggle
orchestration with the live document/revision guards. Use `studio_media_job` to
poll/cancel/save task outputs and `studio_edit` with `ai_import` to place a result.
For uploading reference files, saving/loading named reference sets, page snapshots
and live result placement this main adapter is the appropriate choice. Send the
returned ordered `reference_ids` array to multi-reference recipes; the standalone
server accepts existing document-scoped IDs but does not manage the local library. See [agent control](AGENT-CONTROL.md).

Both servers support per-request MCP `2026-07-28` and negotiated legacy
`2025-11-25`. Installing a server does not itself grant a Kaggle account, GPU quota
or model access. No agent configuration files are silently modified.
