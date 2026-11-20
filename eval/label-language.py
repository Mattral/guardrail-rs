#!/usr/bin/env python3
"""Add a `lang` field (en, de, mixed or unknown) to an evaluation JSONL file.

    python3 eval/label-language.py eval/data/deepset-prompt-injections.jsonl

Deterministic and standard-library only, so anyone gets the same labels:

1. Lowercase the text and split it into words (letters only, so "don't"
   gives "don" and "t").
2. Count the words in ENGLISH and in GERMAN below (common function words).
   Words in both lists, such as "an" or "was", count for neither.
   Each word containing ä, ö, ü or ß also counts once for German.
3. Both counts at least 2: `mixed`. Otherwise the larger count wins; a tie
   (including 0-0) is `unknown`.

It is a heuristic, not a language identifier: short keyword queries with no
function words come out `unknown`, and a single stray word can tip a short
text. eval/README.md gives a hand-checked accuracy estimate.
"""

import json
import re
import sys

ENGLISH = set(
    """the a an and or but is are was were be been being am do does did done
    have has had will would can could should shall may might must not no
    yes of to for with from by on at as about into than then this that these
    those it its i me my you your he she we they them our their his her
    what which who whom whose how why where when there here all any some
    every each just only also very please now new now forget ignore say
    write tell print give make let are""".split()
)
GERMAN = set(
    """der das den dem des und oder aber ist sind war waren sein bin bist
    wird werden wurde wurden hat haben hatte hatten kann können konnte
    sollte sollten muss müssen darf nicht kein keine keinen ja nein von zu
    zum zur für mit aus bei nach über unter auf an als um durch gegen ohne
    dies diese dieser dieses es ich mich mir du dich dir er sie wir ihr
    ihnen sein seine ihre unser unsere was welche welcher welches wer wen
    wem wie warum wo wann dort hier alle alles jede jeder jedes nur auch
    sehr bitte jetzt neue neuen neu vergiss vergessen ignoriere sag sage
    schreib schreibe gib mache lass gibt gab ein eine einen einem einer
    eines im ins vom beim zum noch schon die""".split()
)
SHARED = ENGLISH & GERMAN
ENGLISH -= SHARED
GERMAN -= SHARED
UMLAUT = re.compile(r"[äöüß]")
WORD = re.compile(r"[^\W\d_]+")


def label(text: str) -> str:
    words = WORD.findall(text.lower())
    en = sum(1 for w in words if w in ENGLISH)
    de = sum(1 for w in words if w in GERMAN or UMLAUT.search(w))
    if en >= 2 and de >= 2:
        return "mixed"
    if en > de:
        return "en"
    if de > en:
        return "de"
    return "unknown"


def main(path: str) -> None:
    with open(path, encoding="utf-8") as f:
        records = [json.loads(line) for line in f if line.strip()]
    with open(path, "w", encoding="utf-8") as f:
        for record in records:
            record["lang"] = label(record["text"])
            ordered = {k: record[k] for k in ("id", "label", "lang", "text")}
            f.write(json.dumps(ordered, ensure_ascii=False) + "\n")
    counts = {}
    for record in records:
        counts[record["lang"]] = counts.get(record["lang"], 0) + 1
    print(path, dict(sorted(counts.items())))


if __name__ == "__main__":
    main(sys.argv[1])
