window.BENCHMARK_DATA = {
  "lastUpdate": 1791993994354,
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
          "id": "525ec3b3b51a1920e7da7cc325e7bea434c0e01e",
          "message": "docs: update with GuardRail Studio",
          "timestamp": "2026-09-13T04:48:14+09:00",
          "tree_id": "0641e24651687bac6a478a26246df1d79509a906",
          "url": "https://github.com/Mattral/guardrail-rs/commit/525ec3b3b51a1920e7da7cc325e7bea434c0e01e"
        },
        "date": 1789242710248,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 273,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3140,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 12521,
            "range": "± 224",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 24462,
            "range": "± 485",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 985,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 11624,
            "range": "± 345",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 45097,
            "range": "± 802",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 89681,
            "range": "± 1564",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3170,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 12903,
            "range": "± 163",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "37cb5d7e4308f75b99b9a9689dd4b6598d7011a2",
          "message": "fix(ci): patch vulnerable deps, fix clippy 1.99 lints, make nightly audit fail",
          "timestamp": "2026-09-12T19:48:18Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/18/commits/37cb5d7e4308f75b99b9a9689dd4b6598d7011a2"
        },
        "date": 1791987602683,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 350,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3672,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14311,
            "range": "± 214",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28467,
            "range": "± 545",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1278,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12870,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49653,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 98712,
            "range": "± 174",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3712,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14373,
            "range": "± 98",
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
          "id": "2f98b60cb956d6ab5245f58cf03c6fdfe592b1d7",
          "message": "Merge pull request #18 from Mattral/fix/ci-green\n\nfix(ci): patch vulnerable deps, fix clippy 1.99 lints, make nightly audit fail",
          "timestamp": "2026-10-14T23:39:35+09:00",
          "tree_id": "9cbe950bc2de72eed4daf82eed14b50400907a70",
          "url": "https://github.com/Mattral/guardrail-rs/commit/2f98b60cb956d6ab5245f58cf03c6fdfe592b1d7"
        },
        "date": 1791988935633,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 210,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 2425,
            "range": "± 136",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 9547,
            "range": "± 576",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 18831,
            "range": "± 1134",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 796,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 9365,
            "range": "± 1018",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 36323,
            "range": "± 1057",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 74620,
            "range": "± 4650",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 2502,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 10360,
            "range": "± 277",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "a3e32ee56ec3a695ffc38c5367e1ba9f1a4dfd6a",
          "message": "Land #19, #20, #21 on main (merged into the stacked base branch)",
          "timestamp": "2026-10-14T14:39:41Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/24/commits/a3e32ee56ec3a695ffc38c5367e1ba9f1a4dfd6a"
        },
        "date": 1791990380523,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 359,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3639,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14282,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28396,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1321,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12811,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49296,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 97811,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3654,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14163,
            "range": "± 45",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "4f2fa153ae0d863f74e950ddabc9ffe9fb954f69",
          "message": "Land #19, #20, #21 on main (merged into the stacked base branch)",
          "timestamp": "2026-10-14T14:39:41Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/24/commits/4f2fa153ae0d863f74e950ddabc9ffe9fb954f69"
        },
        "date": 1791990661737,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 205,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 2418,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 9538,
            "range": "± 479",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 18733,
            "range": "± 669",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 783,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 9330,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 36652,
            "range": "± 597",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 72716,
            "range": "± 974",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 2440,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 9828,
            "range": "± 148",
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
          "id": "4e52a3c4ca29ea77d96ea3f855a69f7560835181",
          "message": "Merge pull request #24 from Mattral/fix/ci-green\n\nLand #19, #20, #21 on main (merged into the stacked base branch)",
          "timestamp": "2026-10-15T00:14:12+09:00",
          "tree_id": "38cfaa7590d569c7f22c253612d6588df33b9cc1",
          "url": "https://github.com/Mattral/guardrail-rs/commit/4e52a3c4ca29ea77d96ea3f855a69f7560835181"
        },
        "date": 1791990988322,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 360,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3644,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14297,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28431,
            "range": "± 343",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1309,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12883,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49593,
            "range": "± 369",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 98389,
            "range": "± 394",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3691,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14428,
            "range": "± 117",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "3b09db2140f434b835758481e84ebeeec60b4e46",
          "message": "docs: say that streaming responses skip output redaction (checklist #3)",
          "timestamp": "2026-10-14T15:14:24Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/22/commits/3b09db2140f434b835758481e84ebeeec60b4e46"
        },
        "date": 1791991054208,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 210,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 2367,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 9357,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 18705,
            "range": "± 180",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 752,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 9011,
            "range": "± 387",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 34219,
            "range": "± 1212",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 69717,
            "range": "± 1523",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 2405,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 9979,
            "range": "± 132",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "cc8d62842832e1f3b79cc2d1fe7c72afba6b5fa6",
          "message": "fix(ci): declare the real MSRV (1.88) and make the MSRV job actually test it",
          "timestamp": "2026-10-14T15:14:24Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/23/commits/cc8d62842832e1f3b79cc2d1fe7c72afba6b5fa6"
        },
        "date": 1791991184492,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 364,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3640,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14277,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28405,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1289,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12897,
            "range": "± 152",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49624,
            "range": "± 203",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 98668,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3676,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14316,
            "range": "± 55",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "ce82aaecb7d5332073777757d7a878bb668548de",
          "message": "fix(onnx): run the ONNX stages in CI and fix what that found (checklist #2)",
          "timestamp": "2026-10-14T15:14:24Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/25/commits/ce82aaecb7d5332073777757d7a878bb668548de"
        },
        "date": 1791991289410,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 358,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3657,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14344,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28492,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1286,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12792,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49402,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 97955,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3688,
            "range": "± 360",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14267,
            "range": "± 31",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "80687d39e11c4401a5a9b19b121cd3dceb7d478d",
          "message": "feat(proxy): count stage errors and record fail-open in the audit log (checklist #4)",
          "timestamp": "2026-10-14T15:14:24Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/26/commits/80687d39e11c4401a5a9b19b121cd3dceb7d478d"
        },
        "date": 1791991354681,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 287,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3152,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 12319,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 24491,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1152,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 11079,
            "range": "± 324",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 42906,
            "range": "± 946",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 85438,
            "range": "± 376",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3215,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 12136,
            "range": "± 45",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "4c55a2e3ffdc41cbf3acd4cbfff639a27ea3ab59",
          "message": "refactor: keep one copy of the bundled injection rules (checklist #11)",
          "timestamp": "2026-10-14T15:14:24Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/27/commits/4c55a2e3ffdc41cbf3acd4cbfff639a27ea3ab59"
        },
        "date": 1791991439697,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 213,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 2423,
            "range": "± 147",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 9534,
            "range": "± 490",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 18961,
            "range": "± 811",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 814,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 9092,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 35492,
            "range": "± 1335",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 70159,
            "range": "± 3228",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 2393,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 9962,
            "range": "± 1274",
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
          "id": "68bd582623c884d4ee522cb037bce12619dbb9e7",
          "message": "Merge pull request #23 from Mattral/fix/msrv\n\nfix(ci): declare the real MSRV (1.88) and make the MSRV job actually test it",
          "timestamp": "2026-10-15T00:22:27+09:00",
          "tree_id": "e26fd8124d66bccdf99110726233f6042256bcd9",
          "url": "https://github.com/Mattral/guardrail-rs/commit/68bd582623c884d4ee522cb037bce12619dbb9e7"
        },
        "date": 1791991561306,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 366,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3648,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14301,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28547,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1274,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12872,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49720,
            "range": "± 401",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 98054,
            "range": "± 316",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3664,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14262,
            "range": "± 403",
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
          "id": "2741aadc299b9715c9d3a4d46ceea3900044527d",
          "message": "Merge pull request #25 from Mattral/fix/onnx-tests\n\nfix(onnx): run the ONNX stages in CI and fix what that found (checklist #2)",
          "timestamp": "2026-10-15T00:34:48+09:00",
          "tree_id": "25d1e56a221ced48d62cc28d86b09358726ab94f",
          "url": "https://github.com/Mattral/guardrail-rs/commit/2741aadc299b9715c9d3a4d46ceea3900044527d"
        },
        "date": 1791992231081,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 346,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3650,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14318,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28459,
            "range": "± 404",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1304,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12872,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49529,
            "range": "± 620",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 98402,
            "range": "± 1081",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3685,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14578,
            "range": "± 60",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "44aa04c87ced1309ee44760ba57c276b1481ecfe",
          "message": "feat(proxy): count stage errors and record fail-open in the audit log (checklist #4)",
          "timestamp": "2026-10-14T15:34:57Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/26/commits/44aa04c87ced1309ee44760ba57c276b1481ecfe"
        },
        "date": 1791992443580,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 201,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 2386,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 9535,
            "range": "± 225",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 19153,
            "range": "± 390",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 765,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 8981,
            "range": "± 201",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 34754,
            "range": "± 879",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 69218,
            "range": "± 889",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 2405,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 9812,
            "range": "± 193",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "465d2534ba98ca8e9e80b1a66c53bb351d3634ed",
          "message": "refactor: keep one copy of the bundled injection rules (checklist #11)",
          "timestamp": "2026-10-14T15:34:57Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/27/commits/465d2534ba98ca8e9e80b1a66c53bb351d3634ed"
        },
        "date": 1791992533163,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 374,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 4094,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 16049,
            "range": "± 283",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 31948,
            "range": "± 518",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1417,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 14269,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 55462,
            "range": "± 433",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 110379,
            "range": "± 338",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 4155,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 15727,
            "range": "± 159",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "baaa7abfe28594938434af02262aa96b9b929874",
          "message": "perf(pii): ~2x faster redaction, scan once per pattern (checklist #7)",
          "timestamp": "2026-10-14T15:34:57Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/28/commits/baaa7abfe28594938434af02262aa96b9b929874"
        },
        "date": 1791992671330,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 324,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3458,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 13516,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 26786,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 447,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 5147,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 19946,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/4079B",
            "value": 27128,
            "range": "± 285",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 39696,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3536,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 6357,
            "range": "± 14",
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
          "id": "728a1eda02411f6e348c9bdbeb2568690bb4c319",
          "message": "Merge pull request #27 from Mattral/refactor/single-injection-rules\n\nrefactor: keep one copy of the bundled injection rules (checklist #11)",
          "timestamp": "2026-10-15T00:42:47+09:00",
          "tree_id": "ab7de8caf927d16cd67d0c38fdaa7fa7605ab4c6",
          "url": "https://github.com/Mattral/guardrail-rs/commit/728a1eda02411f6e348c9bdbeb2568690bb4c319"
        },
        "date": 1791992714729,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 351,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3658,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14356,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28494,
            "range": "± 693",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 1277,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 12819,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 49364,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 98056,
            "range": "± 151",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3704,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 14141,
            "range": "± 49",
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
          "id": "b610ed66d29613547398429ccf9c1df544f52194",
          "message": "Merge pull request #28 from Mattral/perf/pii-single-pass\n\nperf(pii): ~2x faster redaction, scan once per pattern (checklist #7)",
          "timestamp": "2026-10-15T00:48:49+09:00",
          "tree_id": "8b9f7a59c4273ecd0aa4dcd1559c6edf37c98871",
          "url": "https://github.com/Mattral/guardrail-rs/commit/b610ed66d29613547398429ccf9c1df544f52194"
        },
        "date": 1791993127385,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 304,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3208,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 12406,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 24712,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 359,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 4363,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 17274,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/4079B",
            "value": 24640,
            "range": "± 501",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 34669,
            "range": "± 567",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3199,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 5419,
            "range": "± 25",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "37ba5b944b01e26a6af1483ca806b763c725ae45",
          "message": "fix(config): make the remaining documented-but-ignored settings work",
          "timestamp": "2026-10-14T15:48:56Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/29/commits/37ba5b944b01e26a6af1483ca806b763c725ae45"
        },
        "date": 1791993334028,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 364,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3664,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14321,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28500,
            "range": "± 203",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 456,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 5492,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 21695,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/4079B",
            "value": 29543,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 43500,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3699,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 6948,
            "range": "± 40",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "d1149cc8775046e90e2f1900950e8fc87dc7ce34",
          "message": "docs: measured latency claims, drop vanity badge and 'production-grade' (checklist #6, #9, #10)",
          "timestamp": "2026-10-14T15:48:56Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/30/commits/d1149cc8775046e90e2f1900950e8fc87dc7ce34"
        },
        "date": 1791993911473,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 349,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3687,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14459,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28751,
            "range": "± 319",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 476,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 5520,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 21707,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/4079B",
            "value": 29403,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 43328,
            "range": "± 208",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3696,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 6769,
            "range": "± 112",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "committer": {
            "name": "Mattral",
            "username": "Mattral"
          },
          "id": "729aa05010d626eeeaf9fe8253d3c1208fa58b05",
          "message": "ci: dry-run publish the whole workspace together",
          "timestamp": "2026-10-14T15:48:56Z",
          "url": "https://github.com/Mattral/guardrail-rs/pull/31/commits/729aa05010d626eeeaf9fe8253d3c1208fa58b05"
        },
        "date": 1791993981053,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 346,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3664,
            "range": "± 155",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 14324,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 28509,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 452,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 5516,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 20747,
            "range": "± 493",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/4079B",
            "value": 29492,
            "range": "± 730",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 43501,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3701,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 6889,
            "range": "± 64",
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
          "id": "f42e2c78a85f917cac326d334c9fd74ad91b1f07",
          "message": "Merge pull request #29 from Mattral/fix/wire-ignored-config\n\nfix(config): make the remaining documented-but-ignored settings work",
          "timestamp": "2026-10-15T01:04:03+09:00",
          "tree_id": "b845c34f0df4be9b0bd5585f5a8c4a1247c4b91d",
          "url": "https://github.com/Mattral/guardrail-rs/commit/f42e2c78a85f917cac326d334c9fd74ad91b1f07"
        },
        "date": 1791993993445,
        "tool": "cargo",
        "benches": [
          {
            "name": "regex_injection_scanner/64B",
            "value": 287,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/1024B",
            "value": 3177,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/4096B",
            "value": 12419,
            "range": "± 257",
            "unit": "ns/iter"
          },
          {
            "name": "regex_injection_scanner/8192B",
            "value": 24698,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/46B",
            "value": 369,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/742B",
            "value": 4452,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/2998B",
            "value": 17409,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/4079B",
            "value": 23718,
            "range": "± 523",
            "unit": "ns/iter"
          },
          {
            "name": "pii_redactor/6034B",
            "value": 34831,
            "range": "± 705",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/regex_injection_async_clean",
            "value": 3212,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "stage_evaluate_async/pii_redactor_async_with_pii",
            "value": 5459,
            "range": "± 12",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}