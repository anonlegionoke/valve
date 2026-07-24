#!/bin/bash

echo "Sending JSON test request to proxy at localhost:8080"
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-Valve-Rule: json-strict" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Output a JSON object with a single key \"name\" mapping to your name. DO NOT use markdown formatting or ```json blocks. Output ONLY raw JSON."}],
    "stream": true
  }'
