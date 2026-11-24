#!/usr/bin/env python3
"""Regenerate core/data/en.tsv from the wordfreq English export (CC BY-SA 4.0).

1. Download:
     https://raw.githubusercontent.com/aparrish/wordfreq-en-25000/main/wordfreq-en-25000-log.json
2. Run:
     python scripts/build-english-dictionary.py path\\to\\wordfreq-en-25000-log.json

The list is converted to British spellings: common American variants are
dropped in favour of their British form, while words that are valid in both
varieties (meter/metre, program/programme, ...) are kept.
"""

import json
import math
import os
import re
import sys
from pathlib import Path

WORD_RE = re.compile(r"^[a-z]{1,24}$")
DEFAULT_COUNT = 20_000

KEEP_BOTH = {
	"meter", "meters", "program", "programs", "tire", "tires", "draft",
	"drafts", "check", "checks", "license", "licenses", "practice",
	"practices", "curb", "curbs", "gray", "grays", "aging", "ax", "axe",
	"mom", "moms", "math", "rigor", "rigors",
}

CURATED = [
	("center", "centre"),
	("centers", "centres"),
	("centered", "centred"),
	("centering", "centring"),
	("theater", "theatre"),
	("theaters", "theatres"),
	("amphitheater", "amphitheatre"),
	("amphitheaters", "amphitheatres"),
	("liter", "litre"),
	("liters", "litres"),
	("fiber", "fibre"),
	("fibers", "fibres"),
	("somber", "sombre"),
	("caliber", "calibre"),
	("calibers", "calibres"),
	("saber", "sabre"),
	("sabers", "sabres"),
	("scepter", "sceptre"),
	("scepters", "sceptres"),
	("specter", "spectre"),
	("specters", "spectres"),
	("meager", "meagre"),
	("sepulcher", "sepulchre"),
	("sepulchers", "sepulchres"),
	("louver", "louvre"),
	("louvers", "louvres"),
	("maneuver", "manoeuvre"),
	("maneuvers", "manoeuvres"),
	("maneuvered", "manoeuvred"),
	("maneuvering", "manoeuvring"),
	("maneuverable", "manoeuvrable"),
	("dialog", "dialogue"),
	("dialogs", "dialogues"),
	("catalog", "catalogue"),
	("catalogs", "catalogues"),
	("analog", "analogue"),
	("analogs", "analogues"),
	("monolog", "monologue"),
	("monologs", "monologues"),
	("travelog", "travelogue"),
	("travelogs", "travelogues"),
	("epilog", "epilogue"),
	("epilogs", "epilogues"),
	("prolog", "prologue"),
	("prologs", "prologues"),
	("demagog", "demagogue"),
	("pedagog", "pedagogue"),
	("synagog", "synagogue"),
	("aluminum", "aluminium"),
	("jewelry", "jewellery"),
	("pajamas", "pyjamas"),
	("omelet", "omelette"),
	("omelets", "omelettes"),
	("mustache", "moustache"),
	("mustaches", "moustaches"),
	("skeptic", "sceptic"),
	("skeptics", "sceptics"),
	("skeptical", "sceptical"),
	("skepticism", "scepticism"),
	("plow", "plough"),
	("plows", "ploughs"),
	("plowed", "ploughed"),
	("plowing", "ploughing"),
	("mold", "mould"),
	("molds", "moulds"),
	("molded", "moulded"),
	("molding", "moulding"),
	("smolder", "smoulder"),
	("smolders", "smoulders"),
	("smoldered", "smouldered"),
	("smoldering", "smouldering"),
	("behoove", "behove"),
	("behooves", "behoves"),
	("behooved", "behoved"),
	("fulfill", "fulfil"),
	("fulfills", "fulfils"),
	("fulfillment", "fulfilment"),
	("enroll", "enrol"),
	("enrolls", "enrols"),
	("enrollment", "enrolment"),
	("enrollments", "enrolments"),
	("skillful", "skilful"),
	("skillfully", "skilfully"),
	("skillfulness", "skilfulness"),
	("willful", "wilful"),
	("willfully", "wilfully"),
	("willfulness", "wilfulness"),
	("installment", "instalment"),
	("installments", "instalments"),
	("marvelous", "marvellous"),
	("marvelously", "marvellously"),
	("license", "licence"),
	("licenses", "licences"),
	("practice", "practise"),
	("practices", "practises"),
	("program", "programme"),
	("programs", "programmes"),
	("meter", "metre"),
	("meters", "metres"),
	("tire", "tyre"),
	("tires", "tyres"),
	("draft", "draught"),
	("drafts", "draughts"),
	("check", "cheque"),
	("checks", "cheques"),
	("curb", "kerb"),
	("curbs", "kerbs"),
	("gray", "grey"),
	("aging", "ageing"),
	("ax", "axe"),
	("mom", "mum"),
	("math", "maths"),
	("rigor", "rigour"),
]

IZE_SUFFIXES = (
	"izations",
	"ization",
	"izable",
	"izers",
	"izer",
	"izing",
	"ized",
	"izes",
	"ize",
)
YZE_SUFFIXES = ("yzed", "yzing", "yzes", "yze")
L_SUFFIXES = ("ed", "ing", "er", "ers")


def load_frequencies(path: Path) -> dict[str, float]:
	freq: dict[str, float] = {}
	with path.open(encoding="utf-8") as source:
		rows = json.load(source)
	for row in rows:
		word = str(row[0]).lower()
		if not WORD_RE.match(word):
			continue
		log_freq = float(row[1])
		if log_freq >= 0:
			continue
		if word not in freq or freq[word] < log_freq:
			freq[word] = log_freq
	return freq


def build(freq: dict[str, float]) -> dict[str, float]:
	drop: set[str] = set()
	boost: dict[str, float] = {}

	def promote(american: str, british: str, *, add: bool = False) -> None:
		if american not in freq:
			return
		if british not in freq:
			if not add:
				return
			freq[british] = freq[american]
		elif freq[british] < freq[american]:
			freq[british] = freq[american]
		if american not in KEEP_BOTH:
			drop.add(american)

	for word in list(freq):
		if len(word) >= 4:
			index = word.rfind("or")
			if index >= 0:
				promote(word, word[:index] + "our" + word[index + 2:])
		for suffix in IZE_SUFFIXES:
			if word.endswith(suffix) and len(word) >= len(suffix) + 3:
				stem = word[: -len(suffix)]
				promote(word, stem + suffix.replace("z", "s"))
		for suffix in YZE_SUFFIXES:
			if word.endswith(suffix) and len(word) >= len(suffix) + 3:
				stem = word[: -len(suffix)]
				promote(word, stem + suffix.replace("z", "s"))
		if re.match(r"^(defen|offen|preten)se(s)?$", word):
			promote(word, word[:-2] + "ce")
		for suffix in L_SUFFIXES:
			if word.endswith(suffix):
				stem = word[: -len(suffix)]
				if len(stem) >= 3 and stem.endswith("l") and not stem.endswith("ll"):
					promote(word, stem + "l" + suffix)

	for word in drop:
		freq.pop(word, None)
	for word, value in boost.items():
		if word not in freq or freq[word] < value:
			freq[word] = value

	drop.clear()
	for american, british in CURATED:
		promote(american, british, add=True)
	for word in drop:
		freq.pop(word, None)
	return freq


def write_dictionary(freq: dict[str, float], path: Path, count: int) -> int:
	ranked = sorted(freq.items(), key=lambda item: (-item[1], item[0]))
	selected = ranked[:count]
	lines = []
	for word, log_freq in selected:
		per_million = math.exp(log_freq) * 1_000_000
		score = round(math.log(per_million + 1) * 100)
		lines.append(f"{word}\t{score}\t0")
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
	return len(selected)


def main() -> int:
	if len(sys.argv) not in (1, 2, 3):
		print("usage: build-english-dictionary.py [wordfreq.json] [count]", file=sys.stderr)
		return 2
	root = Path(__file__).resolve().parent.parent
	default_source = (
		Path(os.environ.get("TEMP", "."))
		/ "opencode"
		/ "wordfreq"
		/ "wf.json"
	)
	source = Path(sys.argv[1]) if len(sys.argv) >= 2 else default_source
	count = int(sys.argv[2]) if len(sys.argv) == 3 else DEFAULT_COUNT
	freq = build(load_frequencies(source))
	written = write_dictionary(freq, root / "core" / "data" / "en.tsv", count)
	print(f"entries: {written}")
	print(f"size: {(root / 'core' / 'data' / 'en.tsv').stat().st_size} bytes")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
