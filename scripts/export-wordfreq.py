#!/usr/bin/env python3
"""Export the full English wordfreq list as JSON for the dictionary builder.

Requires the ``wordfreq`` package:

     python -m pip install wordfreq
     python scripts/export-wordfreq.py path\\to\\wf.json

Each row is ``[word, ln(probability)]``, matching the format consumed by
``scripts/build-english-dictionary.py``.
"""

import json
import math
import os
import sys
from pathlib import Path

import wordfreq


def main() -> int:
	if len(sys.argv) > 2:
		print("usage: export-wordfreq.py [output.json]", file=sys.stderr)
		return 2
	root = Path(__file__).resolve().parent.parent
	default_out = (
		Path(os.environ.get("TEMP", "."))
		/ "opencode"
		/ "wordfreq"
		/ "wf.json"
	)
	out = Path(sys.argv[1]) if len(sys.argv) == 2 else default_out
	freq = wordfreq.get_frequency_dict("en", "best")
	rows = [
		[word, math.log(probability)]
		for word, probability in freq.items()
		if probability > 0
	]
	out.parent.mkdir(parents=True, exist_ok=True)
	out.write_text(json.dumps(rows, ensure_ascii=False), encoding="utf-8")
	print(f"exported {len(rows)} tokens to {out}")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
