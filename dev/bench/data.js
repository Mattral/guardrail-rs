window.BENCHMARK_DATA = {
  "lastUpdate": 1783167087497,
  "repoUrl": "https://github.com/Mattral/guardrail-rs",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "88831350+Mattral@users.noreply.github.com",
            "name": "Min Htet Myet",
            "username": "Mattral"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "abcc5fdf0fe519164b2ebbccf79fff6ddce59c5c",
          "message": "Update GitHub Actions permissions and token usage",
          "timestamp": "2026-07-04T21:05:29+09:00",
          "tree_id": "74092269aa4285ef0defb630b71a50465fffd768",
          "url": "https://github.com/Mattral/guardrail-rs/commit/abcc5fdf0fe519164b2ebbccf79fff6ddce59c5c"
        },
        "date": 1783166903715,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 368,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 4061,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 15941,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 31681,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1576,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 15076,
            "range": "± 239",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 58665,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 116913,
            "range": "± 428",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 4114,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 16038,
            "range": "± 183",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "88831350+Mattral@users.noreply.github.com",
            "name": "Min Htet Myet",
            "username": "Mattral"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a20767d058263d9e216d7d5ca3656b1c83669885",
          "message": "Improve coverage enforcement in CI workflow\n\nUpdated the coverage enforcement script to robustly parse output and handle errors.",
          "timestamp": "2026-07-04T21:09:16+09:00",
          "tree_id": "f63eee70408e341c87afd9cd9ddabe27e3dcb2d5",
          "url": "https://github.com/Mattral/guardrail-rs/commit/a20767d058263d9e216d7d5ca3656b1c83669885"
        },
        "date": 1783167087140,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 332,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3470,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 13552,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 26829,
            "range": "± 853",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1196,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12165,
            "range": "± 235",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 47373,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 94219,
            "range": "± 388",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3507,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 13282,
            "range": "± 70",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}