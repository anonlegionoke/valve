# 🚰 Valve

**A Streaming Token-Level Grammar Constraints Engine.**

Valve is a high-performance, headless HTTP proxy written in Rust that enforces strict schema constraints on Large Language Model (LLM) outputs in real-time. 

## The Core Problem
Existing guardrail proxies wait for the LLM to finish generating text before they parse the JSON, find a formatting error, and throw the response away. This wastes seconds of user time, API costs, and computing power.

## The Solution
Valve acts as a middleman between your application and an LLM provider (like OpenAI). It perfectly mimics the OpenAI Chat API structure, so your frontend app never knows the proxy is there. 

Instead of waiting for completion, Valve parses the LLM token stream atomically (token-by-token). Every time a new token arrives via Server-Sent Events (SSE), Valve checks if it conforms to a complex regex state machine. 

If the LLM generates a character that violates your schema, **Valve instantly kills the stream and forcefully closes the connection at that exact token.**

## Why Rust?
This requires microsecond execution speeds. Every time a new token arrives (every 10–20 milliseconds), the proxy must perform regex Finite State Machine (FSM) transitions on a live token stream. Rust provides the raw performance necessary to do this without introducing any noticeable lag to the end user.

---

## 🚀 Getting Started

### 1. Requirements
- Rust (Cargo)
- Upstream LLM API key (e.g., OpenAI)

### 2. Configuration
Create a `config.toml` file in the root directory. You can define your target endpoint, API key, and the specific regex rule you want to enforce.

```toml
target_endpoint = "https://api.openai.com/v1/chat/completions"
api_key = "sk-YOUR_API_KEY_HERE"

# Example: A strict rule that only allows typical alphabetic streams
rule = "^[A-Za-z0-9 .,!?]*$"
```

### 3. Run the Proxy
Start the proxy server via the command line:

```bash
cargo run
```
*(By default, the server binds to port 8080. You can change this with `--port`).*

### 4. Test It
Send a standard OpenAI Chat Completion request to `localhost:8080`. Valve perfectly proxies the request upstream.

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Say hello!"}],
    "stream": true
  }'
```

If the LLM attempts to generate any character outside your defined regex rule, Valve will instantly print a bright red terminal alert and abort the stream.

## 🛠️ Tech Stack
- **Axum & Tokio** - Asynchronous web server and routing.
- **Reqwest** - Upstream API proxying.
- **EventSource-Stream** - Real-time SSE token interception.
- **Regex** - The constraints engine.
- **Clap, Serde, Toml** - Configuration and CLI management.
