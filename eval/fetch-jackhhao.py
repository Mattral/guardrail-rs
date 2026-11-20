#!/usr/bin/env python3
"""Fetch jackhhao/jailbreak-classification for the detection evaluation.

Downloads the dataset's full CSV at a pinned revision, checks its SHA-256,
and writes eval/data/external/jackhhao-jailbreak-classification.jsonl in the
same shape as the other evaluation files. Standard library only.

    python3 eval/fetch-jackhhao.py

The dataset is Apache-2.0: https://huggingface.co/datasets/jackhhao/jailbreak-classification
It is not committed (2.1 MB), and the optional DeBERTa model was trained on
it, so its scores for that model are not an independent test.
"""

import csv
import hashlib
import io
import json
import pathlib
import urllib.request

REVISION = "2f2ceeb39658696fd3f462403562b6eea5306287"
URL = (
    "https://huggingface.co/datasets/jackhhao/jailbreak-classification/resolve/"
    f"{REVISION}/default/jailbreak_dataset_full.csv"
)
SHA256 = "79a7b90b0abe00e3586cc5048353c3236543cca228a1ee261fe3b57a7cb7e29f"
OUT = (
    pathlib.Path(__file__).resolve().parent
    / "data"
    / "external"
    / "jackhhao-jailbreak-classification.jsonl"
)
LABELS = {"jailbreak": "injection", "benign": "benign"}


def main() -> None:
    request = urllib.request.Request(URL, headers={"User-Agent": "guardrail-rs-eval"})
    with urllib.request.urlopen(request, timeout=60) as response:
        raw = response.read()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != SHA256:
        raise SystemExit(f"SHA-256 mismatch: expected {SHA256}, got {digest}")

    rows = list(csv.DictReader(io.StringIO(raw.decode("utf-8"))))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", encoding="utf-8") as out:
        for i, row in enumerate(rows):
            record = {
                "id": f"jackhhao-{i:04d}",
                "label": LABELS[row["type"]],
                "text": row["prompt"],
            }
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
    print(f"wrote {len(rows)} rows to {OUT}")


if __name__ == "__main__":
    main()
