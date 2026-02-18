#!/bin/bash
# Test OpenRouter API Key
# Usage: ./test-api.sh

echo "=========================================="
echo "Testing OpenRouter API Connection"
echo "=========================================="
echo ""

# Load API key from .env file
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Check if API key is set
if [ -z "$OPENROUTER_API_KEY" ]; then
    echo "ERROR: OPENROUTER_API_KEY not found!"
    echo "Make sure .env file exists and contains the API key."
    exit 1
fi

echo "API Key found: ${OPENROUTER_API_KEY:0:20}..."
echo ""

# Make test request
echo "Sending test request to OpenRouter..."
echo ""

RESPONSE=$(curl -X POST "https://openrouter.ai/api/v1/chat/completions" \
  -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "google/gemini-pro-1.5",
    "messages": [{"role": "user", "content": "Säg hej!"}]
  }' 2>&1)

# Check if response contains error
if echo "$RESPONSE" | grep -q '"error"'; then
    echo "ERROR: API request failed!"
    echo ""
    echo "Response:"
    echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"
    echo ""
    echo "Possible issues:"
    echo "- API key is invalid or expired"
    echo "- OpenRouter account is not active"
    echo "- Rate limit exceeded"
    exit 1
else
    echo "SUCCESS! API is working."
    echo ""
    echo "Response preview:"
    # Extract just the content from the response
    echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['choices'][0]['message']['content'])" 2>/dev/null || echo "$RESPONSE"
    echo ""
    echo "Full response saved to: /tmp/openrouter-test-response.json"
    echo "$RESPONSE" > /tmp/openrouter-test-response.json
fi

echo ""
echo "=========================================="
echo "Test complete"
echo "=========================================="
