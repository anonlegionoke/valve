#!/bin/bash

echo "Sending test request to proxy at localhost:8080"
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Say hello!"}],
    "stream": true
  }'
