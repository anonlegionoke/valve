# 🚰 Valve

Valve is a high-performance, headless HTTP proxy written in Rust that sits between your application and your LLM providers. It enforces strict grammar, regex, and JSON schema constraints on **streaming** LLM responses in real-time, atomically killing streams that violate your rules.

## Features

- **Microsecond Token Validation**: Validates incoming SSE tokens in <30µs. It adds zero noticeable latency to your streams.
- **Universal LLM Router**: Point your app to `localhost:8080` using the standard OpenAI format. Valve automatically routes `gpt-*` models to OpenAI and `gemini-*` models to Google.
- **Route-Specific Constraints**: Pass a custom HTTP header (`X-Valve-Rule: json-strict`) from your frontend to instantly apply different Regex or JSON Schema validation rules on a per-request basis.
- **True JSON Schema Validation**: Don't rely on fragile Regex for structured data. Provide a standard JSON Schema in your config, and Valve will atomically parse the JSON structure as it streams. If the LLM generates a syntax error or violates the schema, Valve catches it instantly.
- **Self-Healing Streams**: If the LLM hallucinates and breaks your formatting rules, Valve doesn't just drop the connection to your user. It silently terminates the upstream connection, sends a corrective prompt to the LLM, and seamlessly stitches the repaired stream back to your frontend. The end-user never even knows an error occurred.

## Quick Start

1. Create a `.env` file with your API keys:
   ```env
   OPENAI_API_KEY=your_key
   GEMINI_API_KEY=your_key
   ```

2. Configure your constraints and providers in `config.toml`:
   ```toml
   [rules.default]
   regex = "^[A-Za-z0-9 .,!?]*$"

   [rules.email-only]
   regex = "^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\\.[a-zA-Z0-9-.]+$"

   [rules.json-strict]
   schema = { type = "object", properties = { name = { type = "string" } }, required = ["name"] }

   [providers.openai]
   endpoint = "https://api.openai.com/v1/chat/completions"
   api_key = "${OPENAI_API_KEY}"

   [providers.gemini]
   endpoint = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
   api_key = "${GEMINI_API_KEY}"
   ```

3. Run the proxy:
   ```bash
   cargo run --release
   ```

4. Route your requests through Valve:
   ```bash
   curl -X POST http://localhost:8080/v1/chat/completions \
     -H "Content-Type: application/json" \
     -H "X-Valve-Rule: json-strict" \
     -d '{
       "model": "gemini-2.5-flash",
       "messages": [{"role": "user", "content": "Output a JSON object with a single key \"name\" mapping to your name. DO NOT use markdown formatting. Output ONLY raw JSON."}],
       "stream": true
     }'
   ```
