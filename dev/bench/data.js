window.BENCHMARK_DATA = {
  "lastUpdate": 1784114468873,
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
          "id": "4a27b5d071105747e82709fde745029a43ac6ce0",
          "message": "Update badges and links in README.md",
          "timestamp": "2026-07-05T03:03:38+09:00",
          "tree_id": "398986bb3e9a855234b00c77c005651d7f6d5617",
          "url": "https://github.com/Mattral/guardrail-rs/commit/4a27b5d071105747e82709fde745029a43ac6ce0"
        },
        "date": 1783188356443,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 298,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3332,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 13289,
            "range": "± 276",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 26057,
            "range": "± 483",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1044,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12232,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 47860,
            "range": "± 433",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 95098,
            "range": "± 1037",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3359,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 13529,
            "range": "± 148",
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
          "id": "8a8bc3af70e460fd38f748efbe80e47d267c2f9f",
          "message": "docs+changelog: add entry for benchmark updates; rewrite README anchors and ensure links resolve correctly",
          "timestamp": "2026-07-05T14:26:58+09:00",
          "tree_id": "ccabdf912ae9ee25b226919cbc31bf55ddef8607",
          "url": "https://github.com/Mattral/guardrail-rs/commit/8a8bc3af70e460fd38f748efbe80e47d267c2f9f"
        },
        "date": 1783229354246,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 357,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3652,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14337,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28457,
            "range": "± 223",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1356,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 13102,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 50691,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 100739,
            "range": "± 484",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3704,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14653,
            "range": "± 94",
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
          "id": "34e9e982fabc7de9cb459d6aa5dd2b0c9f526a9f",
          "message": "changelog: document benchmark updates and label correction; note pipeline latency snapshot inclusion",
          "timestamp": "2026-07-05T19:34:12+09:00",
          "tree_id": "8a482cce1d9b7db84e5017d88a1465f9c243e538",
          "url": "https://github.com/Mattral/guardrail-rs/commit/34e9e982fabc7de9cb459d6aa5dd2b0c9f526a9f"
        },
        "date": 1783247792674,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 369,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 4042,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 15869,
            "range": "± 238",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 31519,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1568,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 14650,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 56377,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 112463,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 4105,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 16015,
            "range": "± 34",
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
          "id": "d0fb1501d4989a92ecad79a36601684b5e5f3c74",
          "message": "fix: Enhance README badges and add Colab link\n\nUpdated badge styles in README for better visibility and added a link to open in Colab.",
          "timestamp": "2026-07-05T19:44:51+09:00",
          "tree_id": "5843a45d726b02973b7093efc20df66ba894ac79",
          "url": "https://github.com/Mattral/guardrail-rs/commit/d0fb1501d4989a92ecad79a36601684b5e5f3c74"
        },
        "date": 1783248422406,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 358,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3654,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14284,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28442,
            "range": "± 177",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1389,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 13128,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 50747,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 100970,
            "range": "± 473",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3683,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14690,
            "range": "± 40",
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
          "id": "14c1634254223ee0e23f1f66acab4b71fde2f975",
          "message": "fix: Add visitor badge to README",
          "timestamp": "2026-07-05T20:33:39+09:00",
          "tree_id": "25025ab3aa040d900be3d7050b0df584ebcc9b0a",
          "url": "https://github.com/Mattral/guardrail-rs/commit/14c1634254223ee0e23f1f66acab4b71fde2f975"
        },
        "date": 1783251351244,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 356,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3650,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14271,
            "range": "± 334",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28412,
            "range": "± 216",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1415,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 13178,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 50946,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 101228,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3699,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14443,
            "range": "± 81",
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
          "id": "8843cbde23f5ef2e35b59913c8ddfb1df63d49be",
          "message": "fix: Add visitor badge to README",
          "timestamp": "2026-07-15T21:28:03Z",
          "tree_id": "25025ab3aa040d900be3d7050b0df584ebcc9b0a",
          "url": "https://github.com/Mattral/guardrail-rs/commit/8843cbde23f5ef2e35b59913c8ddfb1df63d49be"
        },
        "date": 1784113347717,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 285,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3142,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 12322,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 24486,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1183,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 11333,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 43814,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 87097,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3190,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 12475,
            "range": "± 169",
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
          "id": "14d6fe2708604dc1f074770b86dc8193527e8dd7",
          "message": "chore(ci): update cargo-deny config and apply dependency updates\n\n- Migrated deny.toml to new cargo-deny schema:\n  - Removed deprecated fields (`deny`, `copyleft`, `allow-osi-fsf-free`, `default`)\n  - Cleaned up license allow-list to avoid unmatched warnings\n  - Retained explicit bans for openssl/openssl-sys with wrappers\n\n- Applied cargo update commands to resolve advisories:\n  - Upgraded crossbeam-epoch to >=0.9.20 (RUSTSEC-2026-0204)\n  - Aligned duplicate crate versions via cargo update\n  - Left Cargo.toml untouched for now; overrides will follow separately\n\n- CI: cargo-deny and cargo-audit now pass with updated config and patched deps",
          "timestamp": "2026-07-15T11:18:53Z",
          "tree_id": "658dda88a71cee45bb969fad47a9736bc6e6f2ff",
          "url": "https://github.com/Mattral/guardrail-rs/commit/14d6fe2708604dc1f074770b86dc8193527e8dd7"
        },
        "date": 1784114468100,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 297,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3375,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 13227,
            "range": "± 181",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 26204,
            "range": "± 356",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1035,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12418,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 48382,
            "range": "± 761",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 95977,
            "range": "± 1357",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3381,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 13503,
            "range": "± 223",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}