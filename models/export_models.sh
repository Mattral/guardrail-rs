#!/usr/bin/env bash
#
# Export the recommended ONNX models for guardrail-rs's optional semantic
# classifiers. This is a one-time, offline preparation step; it requires
# Python + the HuggingFace `optimum` package, but the resulting `.onnx` /
# `tokenizer.json` files are consumed entirely by the Rust runtime — no
# Python is needed to run guardrail-rs itself.
#
# Usage:
#   ./models/export_models.sh [output_dir]
#
set -euo pipefail

OUT_DIR="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"

echo "==> Installing export dependencies (optimum[exporters])"
pip install --quiet "optimum[exporters]"

echo "==> Exporting prompt-injection classifier"
optimum-cli export onnx \
  --model protectai/deberta-v3-base-prompt-injection-v2 \
  --task text-classification \
  "${OUT_DIR}/prompt-injection/"

echo "==> Exporting toxicity classifier"
optimum-cli export onnx \
  --model unitary/unbiased-toxic-roberta \
  --task text-classification \
  "${OUT_DIR}/toxicity/"

echo "==> Done. Models written to:"
echo "    ${OUT_DIR}/prompt-injection/"
echo "    ${OUT_DIR}/toxicity/"
echo
echo "Enable these in guardrail.toml under [stages.onnx_injection] and"
echo "[stages.toxicity], then build with: cargo build --features onnx"
