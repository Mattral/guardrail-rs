#!/usr/bin/env bash
# Load test of the proxy with both ONNX stages enabled, against an upstream
# that answers at once (mock_upstream.py), driven by oha.
#
#   cargo build --release -p guardrail-cli --features onnx
#   contrib/load-test/onnx-proxy.sh
#
# Environment (defaults in brackets):
#   MODELS       directory with prompt-injection/ and toxicity/ [models]
#   SESSIONS     [onnx] sessions, per stage; 0 = default        [0]
#   INTRA        [onnx] intra_threads                           [1]
#   CONCURRENCY  oha connections                                [16]
#   REQUESTS     requests measured, after a warm-up             [256]
#   PORT, UPSTREAM_PORT                                         [8089, 9099]
#   PROXY_CPUS   run the proxy on these CPUs only (taskset list, e.g. 0-3);
#                its automatic pool size follows                [all CPUs]
#   LOAD_CPUS    run the mock upstream and oha on these CPUs     [all CPUs]
#
# The prompt is a benign ~1 KB message, so no stage blocks it and every
# request runs both models. Prints oha's report, then the proxy's resident
# and peak memory (Linux).
set -euo pipefail

cd "$(dirname "$0")/../.."
MODELS=$(realpath "${MODELS:-models}")
SESSIONS=${SESSIONS:-0}
INTRA=${INTRA:-1}
CONCURRENCY=${CONCURRENCY:-16}
REQUESTS=${REQUESTS:-256}
PORT=${PORT:-8089}
UPSTREAM_PORT=${UPSTREAM_PORT:-9099}
BIN=target/release/guardrail

# Command prefixes that pin to PROXY_CPUS / LOAD_CPUS (taskset execs, so $!
# is still the pinned process).
proxy_pin=()
load_pin=()
if [ -n "${PROXY_CPUS:-}" ]; then proxy_pin=(taskset -c "$PROXY_CPUS"); fi
if [ -n "${LOAD_CPUS:-}" ]; then load_pin=(taskset -c "$LOAD_CPUS"); fi

work=$(mktemp -d)
pids=()
cleanup() {
    for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
    wait 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

cat > "$work/guardrail.toml" <<EOF
[server]
host = "127.0.0.1"
port = $PORT

[upstream]
url = "http://127.0.0.1:$UPSTREAM_PORT"

[pipeline]
request_stages = ["regex_injection", "onnx_injection", "pii_redactor", "toxicity", "policy"]

[stages.onnx_injection]
enabled = true
model_path = "$MODELS/prompt-injection/model.onnx"
tokenizer_path = "$MODELS/prompt-injection/tokenizer.json"

[stages.toxicity]
enabled = true
model_path = "$MODELS/toxicity/model.onnx"
tokenizer_path = "$MODELS/toxicity/tokenizer.json"

[onnx]
sessions = $SESSIONS
intra_threads = $INTRA

[observability]
log_level = "warn"
EOF

sentence="Please summarize the attached quarterly report for the team. "
prompt=$(printf "%.0s$sentence" $(seq 1 17))
printf '{"model":"load-test","messages":[{"role":"user","content":"%s"}]}' \
    "${prompt% }" > "$work/body.json"

"$BIN" validate --config "$work/guardrail.toml" > /dev/null
"${load_pin[@]}" python3 -I contrib/load-test/mock_upstream.py "$UPSTREAM_PORT" &
pids+=($!)
"${proxy_pin[@]}" "$BIN" run --config "$work/guardrail.toml" &
proxy=$!
pids+=("$proxy")

until curl -fs "http://127.0.0.1:$PORT/healthz" > /dev/null; do
    kill -0 "$proxy" || { echo "proxy exited" >&2; exit 1; }
    sleep 0.5
done

url="http://127.0.0.1:$PORT/v1/chat/completions"
status=$(curl -s -o "$work/first.json" -w '%{http_code}' -H 'content-type: application/json' \
    --data-binary @"$work/body.json" "$url")
if [ "$status" != 200 ]; then
    echo "warm-up request got $status, expected 200:" >&2
    cat "$work/first.json" >&2
    exit 1
fi

load() {
    "${load_pin[@]}" oha --no-tui -n "$1" -c "$CONCURRENCY" -m POST -T application/json -D "$work/body.json" "$url"
}
load "$((CONCURRENCY * 2))" > /dev/null # warm up every session
echo "sessions=$SESSIONS intra_threads=$INTRA concurrency=$CONCURRENCY requests=$REQUESTS" \
    "prompt=$(wc -c < "$work/body.json")B proxy_cpus=${PROXY_CPUS:-all}"
load "$REQUESTS"
grep -E '^Vm(RSS|HWM)' "/proc/$proxy/status"
