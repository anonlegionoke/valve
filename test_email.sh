#!/bin/bash

echo "Sending email test request to proxy at localhost:8080"
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-Valve-Rule: email-only" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Say hello!"}],
    "stream": true
  }'
