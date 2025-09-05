# Moonbot (sunbot-rs fork)

A Discord bot built with serenity + poise, SeaORM, and async-openai. It supports OpenAI and OpenAI-compatible local LLMs (Ollama, vLLM, LM Studio), global prompts editable from Discord, and optional hyper-personalization.

## Quick start

1. Copy `config.toml.example` to `config.toml` and fill in your Discord token.
2. If using a local LLM, set `openai.api_base` to your server and ensure it includes `/v1`.
	- Examples: `http://localhost:11434/v1` (Ollama), `http://localhost:8000/v1` (gateway)
3. Run the bot:
	- Set `MOONBOT_CONFIG_FILE` to the path of your `config.toml` (optional; defaults to `config.toml`).
	- Build and run with Cargo.

Slash commands include `/askgpt`, `/genimage`, `/status`, `/prompt_*`, `/mood`, and `/profile`.

Notes:
- If you previously registered global commands, this fork clears them on startup and registers per-guild to avoid duplicates.
- On Windows, stop the running bot before rebuilding to avoid file locking.
