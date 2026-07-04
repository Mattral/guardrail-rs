window.BENCHMARK_DATA = {
  "lastUpdate": 1783186517118,
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
          "id": "846a1d3f3ad013edab9ee79a6c3ba4cbb61924fc",
          "message": "Modify coverage command to allow errors\n\nEnsure that the coverage generation step does not fail the CI if it encounters errors.",
          "timestamp": "2026-07-04T21:25:23+09:00",
          "tree_id": "995549141c5640afe05614c339dca2f1d1c28174",
          "url": "https://github.com/Mattral/guardrail-rs/commit/846a1d3f3ad013edab9ee79a6c3ba4cbb61924fc"
        },
        "date": 1783168060148,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 357,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3654,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14343,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28474,
            "range": "± 175",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1372,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 13099,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 50824,
            "range": "± 258",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 100986,
            "range": "± 173",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3705,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14725,
            "range": "± 34",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mattralminn@gmail.com",
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "email": "mattralminn@gmail.com",
            "name": "Mattral",
            "username": "Mattral"
          },
          "distinct": true,
          "id": "e31802db84219a4fb0dedfd397bcb883379203b1",
          "message": "infra: update devcontainer, nextest, issue templates, codecov, contrib configs; align with current CI/CD setup",
          "timestamp": "2026-07-05T02:24:08+09:00",
          "tree_id": "5b17b67d9ef3bc19a3a3f91f317646a5612d8149",
          "url": "https://github.com/Mattral/guardrail-rs/commit/e31802db84219a4fb0dedfd397bcb883379203b1"
        },
        "date": 1783185988196,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 370,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 4041,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 15867,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 31544,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1542,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 14548,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 56323,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 112164,
            "range": "± 209",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 4090,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 16035,
            "range": "± 41",
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
            "email": "88831350+Mattral@users.noreply.github.com",
            "name": "Min Htet Myet",
            "username": "Mattral"
          },
          "distinct": true,
          "id": "0d946c51e4c459add1a502521e24378a64d7ffef",
          "message": "Created using Colab",
          "timestamp": "2026-07-05T02:33:01+09:00",
          "tree_id": "98263abac3deea7ba780280fefef599a9b650169",
          "url": "https://github.com/Mattral/guardrail-rs/commit/0d946c51e4c459add1a502521e24378a64d7ffef"
        },
        "date": 1783186516352,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 334,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3477,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 13557,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 26835,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1205,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12028,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 46722,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 93042,
            "range": "± 1001",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3534,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 13324,
            "range": "± 30",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}