#!/usr/bin/env python3
"""Regenerate core/data/es.tsv from the full wordfreq Spanish list.

Requires the ``wordfreq`` package:

     python -m pip install wordfreq
     python scripts/build-spanish-dictionary.py

The list keeps every single-token Spanish form up to 31 characters, including
accents, diaeresis and ñ. Data is CC BY-SA 4.0; see core/data/NOTICE.md.
"""

import math
import re
import sys
from pathlib import Path

import wordfreq

WORD_RE = re.compile(r"^[a-záéíóúüñ]+$")
DEFAULT_COUNT = 0


def write_dictionary(freq: dict[str, float], path: Path, count: int) -> int:
	ranked = sorted(freq.items(), key=lambda item: (-item[1], item[0]))
	selected = ranked[:count] if count > 0 else ranked
	lines = []
	for word, log_freq in selected:
		per_million = math.exp(log_freq) * 1_000_000
		score = round(math.log(per_million + 1) * 100)
		lines.append(f"{word}\t{score}\t0")
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
	return len(selected)


def main() -> int:
	if len(sys.argv) > 2:
		print("usage: build-spanish-dictionary.py [count]", file=sys.stderr)
		print("count 0 (the default) keeps every form", file=sys.stderr)
		return 2
	root = Path(__file__).resolve().parent.parent
	count = int(sys.argv[1]) if len(sys.argv) == 2 else DEFAULT_COUNT
	raw = wordfreq.get_frequency_dict("es", "best")
	freq: dict[str, float] = {}
	for word, probability in raw.items():
		word = word.lower()
		if probability <= 0 or len(word) > 31 or not WORD_RE.match(word):
			continue
		if word not in freq or freq[word] < math.log(probability):
			freq[word] = math.log(probability)
	written = write_dictionary(freq, root / "core" / "data" / "es.tsv", count)
	print(f"entries: {written}")
	print(f"size: {(root / 'core' / 'data' / 'es.tsv').stat().st_size} bytes")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
