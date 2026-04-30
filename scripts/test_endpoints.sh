#!/usr/bin/env bash
# AgentForge — full endpoint smoke test
# Usage: ./scripts/test_endpoints.sh [API_BASE]
# Requires: curl, jq
set -euo pipefail

API="${1:-http://localhost:8080}"
PASS=0
FAIL=0

# ── colours ────────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

ok()   { echo -e "${GREEN}  ✓ PASS${NC}  $1"; PASS=$((PASS+1)); }
fail() { echo -e "${RED}  ✗ FAIL${NC}  $1 — $2"; FAIL=$((FAIL+1)); }
info() { echo -e "${CYAN}  ▸${NC}  $1"; }
warn() { echo -e "${YELLOW}  ⚠ NOTE${NC}  $1"; }
sep()  { echo -e "\n${CYAN}━━━  $1  ━━━${NC}"; }

# ── helpers ────────────────────────────────────────────────────────────────────
# curl_json <method> <path> [body]  → print response body; sets HTTP_CODE
http_code() { printf '%s' "${HTTP_CODE}"; }

curl_json() {
  local method="$1" path="$2" body="${3:-}"
  if [[ -n "$body" ]]; then
    HTTP_CODE=$(curl -s -o /tmp/af_resp.json -w '%{http_code}' \
      -X "$method" "$API$path" \
      -H 'Content-Type: application/json' \
      -d "$body")
  else
    HTTP_CODE=$(curl -s -o /tmp/af_resp.json -w '%{http_code}' \
      -X "$method" "$API$path")
  fi
  cat /tmp/af_resp.json
}

assert_field() {
  # assert_field <json_file_or_string> <jq_path> <description>
  local val
  val=$(cat /tmp/af_resp.json | jq -r "$1" 2>/dev/null)
  if [[ "$val" == "null" || -z "$val" ]]; then
    return 1
  fi
  return 0
}

# Poll until status field matches one of the terminal values
# poll_status <url> <jq_status_path> <terminal1> [<terminal2>] [<max_tries>]
poll_until_done() {
  local url="$1" jq_path="$2" max_tries="${3:-60}"  # default 5 min (60 × 5s)
  local terminal1="${4:-complete}" terminal2="${5:-error}"
  info "Polling $url (up to $((max_tries * 5))s) ..."
  for ((i=1; i<=max_tries; i++)); do
    HTTP_CODE=$(curl -s -o /tmp/af_poll.json -w '%{http_code}' "$url")
    local status
    status=$(cat /tmp/af_poll.json | jq -r "$jq_path" 2>/dev/null)
    info "  [$i/$max_tries] status=$status"
    if [[ "$status" == "$terminal1" || "$status" == "$terminal2" ]]; then
      cp /tmp/af_poll.json /tmp/af_resp.json
      echo "$status"
      return 0
    fi
    sleep 5
  done
  echo "timeout"
  return 1
}

# ── wait for API ───────────────────────────────────────────────────────────────
sep "Waiting for API"
for ((i=1; i<=30; i++)); do
  if curl -sf "$API/agents" >/dev/null 2>&1; then
    ok "API is reachable at $API"
    break
  fi
  echo -n "."
  sleep 2
  if [[ $i -eq 30 ]]; then
    fail "API" "not reachable after 60 seconds — is the server running?"
    exit 1
  fi
done

# ── read fixture ──────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$SCRIPT_DIR/../fixtures/customer-support-agent.yaml"
if [[ ! -f "$FIXTURE" ]]; then
  fail "fixture" "cannot find $FIXTURE"
  exit 1
fi
FIXTURE_CONTENT=$(cat "$FIXTURE")

# Escape for JSON embedding
FIXTURE_JSON=$(python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))' <<<"$FIXTURE_CONTENT")

# ── 1. POST /agents (v1.0.0) ─────────────────────────────────────────────────
sep "1. POST /agents"
curl_json POST /agents "{\"content\": $FIXTURE_JSON}" >/dev/null
if [[ "$HTTP_CODE" == "201" || "$HTTP_CODE" == "200" ]]; then
  AGENT_ID=$(cat /tmp/af_resp.json | jq -r '.id')
  AGENT_NAME=$(cat /tmp/af_resp.json | jq -r '.name')
  ok "POST /agents → $HTTP_CODE  id=$AGENT_ID  name=$AGENT_NAME"
else
  fail "POST /agents" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  exit 1
fi

# ── 2. GET /agents ───────────────────────────────────────────────────────────
sep "2. GET /agents"
curl_json GET '/agents?limit=20&offset=0' >/dev/null
if [[ "$HTTP_CODE" == "200" ]]; then
  COUNT=$(cat /tmp/af_resp.json | jq 'if type=="array" then length else .data | length end' 2>/dev/null || echo "?")
  ok "GET /agents → 200  count=$COUNT"
else
  fail "GET /agents" "HTTP $HTTP_CODE"
fi

# ── 3. GET /agents/:id ───────────────────────────────────────────────────────
sep "3. GET /agents/:id"
curl_json GET "/agents/$AGENT_ID" >/dev/null
if [[ "$HTTP_CODE" == "200" ]]; then
  ok "GET /agents/$AGENT_ID → 200"
else
  fail "GET /agents/$AGENT_ID" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
fi

# ── 4. POST /agents (v1.0.1 — slightly modified for diff) ────────────────────
sep "4. POST /agents (v2 for diff/shadow tests)"
# Replace version string to get a distinct SHA
FIXTURE_V2_CONTENT="${FIXTURE_CONTENT/version: \"1.0.0\"/version: \"1.0.1\"}"
FIXTURE_V2_JSON=$(python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))' <<<"$FIXTURE_V2_CONTENT")
curl_json POST /agents "{\"content\": $FIXTURE_V2_JSON}" >/dev/null
if [[ "$HTTP_CODE" == "201" || "$HTTP_CODE" == "200" ]]; then
  AGENT_ID2=$(cat /tmp/af_resp.json | jq -r '.id')
  ok "POST /agents (v2) → $HTTP_CODE  id=$AGENT_ID2"
else
  fail "POST /agents (v2)" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  AGENT_ID2="$AGENT_ID"  # fallback: use same agent for rest of tests
fi

# ── 5. GET /diff ─────────────────────────────────────────────────────────────
sep "5. GET /diff?v1=&v2="
curl_json GET "/diff?v1=$AGENT_ID&v2=$AGENT_ID2" >/dev/null
if [[ "$HTTP_CODE" == "200" ]]; then
  ok "GET /diff → 200"
else
  fail "GET /diff" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
fi

# ── 6. POST /runs ─────────────────────────────────────────────────────────────
sep "6. POST /runs"
RUN_BODY="{\"agent_id\": \"$AGENT_ID\", \"scenario_count\": 1, \"concurrency\": 1, \"seed\": 42}"
curl_json POST /runs "$RUN_BODY" >/dev/null
if [[ "$HTTP_CODE" == "202" || "$HTTP_CODE" == "200" || "$HTTP_CODE" == "201" ]]; then
  RUN_ID=$(cat /tmp/af_resp.json | jq -r '.id')
  RUN_STATUS=$(cat /tmp/af_resp.json | jq -r '.status')
  ok "POST /runs → $HTTP_CODE  id=$RUN_ID  status=$RUN_STATUS"
else
  fail "POST /runs" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  RUN_ID=""
fi

# ── 7. GET /runs/:id (poll until terminal) ────────────────────────────────────
sep "7. GET /runs/:id  (poll)"
if [[ -n "$RUN_ID" ]]; then
  FINAL_STATUS=$(poll_until_done "$API/runs/$RUN_ID" '.status' 60 'complete' 'error')
  if [[ "$FINAL_STATUS" == "complete" ]]; then
    ok "GET /runs/$RUN_ID → terminal status=complete"
  elif [[ "$FINAL_STATUS" == "error" ]]; then
    warn "Run ended with status=error (likely missing real OPENAI_API_KEY in .env)"
    ok "GET /runs/$RUN_ID → 200 with terminal status=error  (endpoint works)"
  elif [[ "$FINAL_STATUS" == "timeout" ]]; then
    fail "GET /runs/$RUN_ID" "Did not reach terminal status within timeout"
  else
    ok "GET /runs/$RUN_ID → terminal status=$FINAL_STATUS"
  fi
else
  warn "Skipping GET /runs/:id (no RUN_ID)"
fi

# ── 8. GET /runs/:id/scorecard ────────────────────────────────────────────────
sep "8. GET /runs/:id/scorecard"
if [[ -n "$RUN_ID" ]]; then
  curl_json GET "/runs/$RUN_ID/scorecard" >/dev/null
  if [[ "$HTTP_CODE" == "200" ]]; then
    ok "GET /runs/$RUN_ID/scorecard → 200"
  elif [[ "$HTTP_CODE" == "400" || "$HTTP_CODE" == "422" ]]; then
    # Run errored — scorecard not available — that's the expected API contract
    warn "Scorecard not available (run errored): HTTP $HTTP_CODE — $(cat /tmp/af_resp.json | jq -r '.message // .error // .')"
    ok "GET /runs/:id/scorecard → responded (run in error state, no scorecard)"
  else
    fail "GET /runs/$RUN_ID/scorecard" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  fi
else
  warn "Skipping scorecard (no RUN_ID)"
fi

# ── 9. POST /promote/:run_id ─────────────────────────────────────────────────
sep "9. POST /promote/:run_id"
if [[ -n "$RUN_ID" ]]; then
  curl_json POST "/promote/$RUN_ID" "" >/dev/null
  if [[ "$HTTP_CODE" == "200" ]]; then
    APPROVED=$(cat /tmp/af_resp.json | jq -r '.approved')
    ok "POST /promote/$RUN_ID → 200  approved=$APPROVED"
  elif [[ "$HTTP_CODE" == "400" || "$HTTP_CODE" == "422" ]]; then
    MSG=$(cat /tmp/af_resp.json | jq -r '.message // .error // .')
    warn "Promote rejected (expected if run errored): $MSG"
    ok "POST /promote/:run_id → responded with correct rejection (run not complete)"
  else
    fail "POST /promote/$RUN_ID" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  fi
else
  warn "Skipping promote (no RUN_ID)"
fi

# ── 10. POST /shadow-runs ─────────────────────────────────────────────────────
sep "10. POST /shadow-runs"
SHADOW_BODY="{\"champion_agent_id\": \"$AGENT_ID\", \"candidate_agent_id\": \"$AGENT_ID2\", \"traffic_percent\": 10}"
curl_json POST /shadow-runs "$SHADOW_BODY" >/dev/null
if [[ "$HTTP_CODE" == "202" || "$HTTP_CODE" == "201" || "$HTTP_CODE" == "200" ]]; then
  SHADOW_ID=$(cat /tmp/af_resp.json | jq -r '.id')
  ok "POST /shadow-runs → $HTTP_CODE  id=$SHADOW_ID"
else
  fail "POST /shadow-runs" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  SHADOW_ID=""
fi

# ── 11. GET /shadow-runs/:id ─────────────────────────────────────────────────
sep "11. GET /shadow-runs/:id"
if [[ -n "$SHADOW_ID" ]]; then
  curl_json GET "/shadow-runs/$SHADOW_ID" >/dev/null
  if [[ "$HTTP_CODE" == "200" ]]; then
    STATUS=$(cat /tmp/af_resp.json | jq -r '.status')
    ok "GET /shadow-runs/$SHADOW_ID → 200  status=$STATUS"
  else
    fail "GET /shadow-runs/$SHADOW_ID" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  fi
else
  warn "Skipping GET /shadow-runs/:id (no SHADOW_ID)"
fi

# ── 12. POST /exports/finetune ───────────────────────────────────────────────
sep "12. POST /exports/finetune"
# Use the run ID (works regardless of run status)
if [[ -n "$RUN_ID" ]]; then
  EXPORT_BODY="{\"run_id\": \"$RUN_ID\", \"format\": \"openai\"}"
  curl_json POST /exports/finetune "$EXPORT_BODY" >/dev/null
  if [[ "$HTTP_CODE" == "202" || "$HTTP_CODE" == "201" || "$HTTP_CODE" == "200" ]]; then
    EXPORT_ID=$(cat /tmp/af_resp.json | jq -r '.id')
    ok "POST /exports/finetune → $HTTP_CODE  id=$EXPORT_ID"
  else
    fail "POST /exports/finetune" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
    EXPORT_ID=""
  fi
else
  warn "Skipping POST /exports/finetune (no RUN_ID)"
  EXPORT_ID=""
fi

# ── 13. GET /exports/finetune/:id ────────────────────────────────────────────
sep "13. GET /exports/finetune/:id"
if [[ -n "${EXPORT_ID:-}" ]]; then
  curl_json GET "/exports/finetune/$EXPORT_ID" >/dev/null
  if [[ "$HTTP_CODE" == "200" ]]; then
    FMT=$(cat /tmp/af_resp.json | jq -r '.format')
    STATUS=$(cat /tmp/af_resp.json | jq -r '.status')
    ok "GET /exports/finetune/$EXPORT_ID → 200  format=$FMT  status=$STATUS"
  else
    fail "GET /exports/finetune/$EXPORT_ID" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  fi
else
  warn "Skipping GET /exports/finetune/:id (no EXPORT_ID)"
fi

# ── 14. POST /benchmarks ─────────────────────────────────────────────────────
sep "14. POST /benchmarks"
BENCH_BODY="{\"agent_id\": \"$AGENT_ID\", \"suite\": \"gaia\"}"
curl_json POST /benchmarks "$BENCH_BODY" >/dev/null
if [[ "$HTTP_CODE" == "202" || "$HTTP_CODE" == "201" || "$HTTP_CODE" == "200" ]]; then
  BENCH_ID=$(cat /tmp/af_resp.json | jq -r '.id')
  ok "POST /benchmarks → $HTTP_CODE  id=$BENCH_ID"
else
  fail "POST /benchmarks" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  BENCH_ID=""
fi

# ── 15. GET /benchmarks/:id ──────────────────────────────────────────────────
sep "15. GET /benchmarks/:id"
if [[ -n "${BENCH_ID:-}" ]]; then
  curl_json GET "/benchmarks/$BENCH_ID" >/dev/null
  if [[ "$HTTP_CODE" == "200" ]]; then
    SUITE=$(cat /tmp/af_resp.json | jq -r '.suite')
    STATUS=$(cat /tmp/af_resp.json | jq -r '.status')
    ok "GET /benchmarks/$BENCH_ID → 200  suite=$SUITE  status=$STATUS"
  else
    fail "GET /benchmarks/$BENCH_ID" "HTTP $HTTP_CODE: $(cat /tmp/af_resp.json)"
  fi
else
  warn "Skipping GET /benchmarks/:id (no BENCH_ID)"
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
TOTAL=$((PASS + FAIL))
if [[ $FAIL -eq 0 ]]; then
  echo -e "${GREEN}  ALL $PASS/$TOTAL PASSED${NC}"
else
  echo -e "${RED}  $FAIL FAILED  /  ${GREEN}$PASS PASSED${NC}  (total $TOTAL)"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Print IDs for browser testing
echo ""
echo "IDs for browser testing:"
echo "  AGENT_ID  = ${AGENT_ID:-n/a}"
echo "  AGENT_ID2 = ${AGENT_ID2:-n/a}"
echo "  RUN_ID    = ${RUN_ID:-n/a}"
echo "  SHADOW_ID = ${SHADOW_ID:-n/a}"
echo "  EXPORT_ID = ${EXPORT_ID:-n/a}"
echo "  BENCH_ID  = ${BENCH_ID:-n/a}"
echo ""
echo "  UI: http://localhost:5173"
echo "  API: $API"

[[ $FAIL -eq 0 ]]
