#!/usr/bin/env python3
"""Build core/data/ko_conj.tsv from KoParadigm stems and Kiwi joining.

This is a build-time step. It is not needed to build or run the keyboard.

Requirements (pin these versions for reproducible output):

    python -m pip install koparadigm==0.10.0 kiwipiepy==0.25.1 wordfreq

KoParadigm supplies the Korean verb/adjective inventory and conjugation
classes. Kiwi supplies complete surface-form joining, including irregular
allomorphs. The generated forms use the source stem's wordfreq prior with a
small generated-form penalty.

The shipped dictionary is the concatenation of data/ko.tsv and the generated
data/ko_conj.tsv. Forms already present in ko.tsv are not duplicated here.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

try:
	import wordfreq
	from kiwipiepy import Kiwi
	from koparadigm import Paradigm
except ImportError as error:
	print(f"missing build dependency: {error}", file=sys.stderr)
	print(
		"install: python -m pip install koparadigm==0.10.0 kiwipiepy==0.25.1 wordfreq",
		file=sys.stderr,
	)
	raise SystemExit(2)

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / "core" / "data" / "ko.tsv"
SEQUENCES = ROOT / "scripts" / "korean_sequences.tsv"
OUT = ROOT / "core" / "data" / "ko_conj.tsv"

GENERATED_PENALTY = 0.25
DERIVED_PENALTY = 0.25
GENERATED_FLAG = 16
STEM_TAGS = {"VV", "VA", "VX", "VCP", "VCN"}
NOUN_TAGS = {"NNG", "NNP", "XSN"}
DERIVATIONS = (
	("하", "noun+하"),
	("되", "noun+되"),
	("시키", "noun+시키"),
	("스럽", "noun+스럽"),
	("롭", "noun+롭"),
)

# `하` + `여` remains a valid uncontracted formal/literary spelling. Kiwi joins
# the ordinary contracted forms (`해요`) for `하` + `어요`; these add the less
# common uncontracted variants without a separate stem-specific generator.
HA_SEQUENCES = (
	[("여", "EC")],
	[("여서", "EC")],
	[("여도", "EC")],
	[("여야", "EC")],
	[("여요", "EF")],
)


def base_words() -> set[str]:
	words = set()
	for line in BASE.read_text(encoding="utf-8").splitlines():
		word = line.split("\t", 1)[0]
		if word:
			words.add(word)
	return words


def read_sequences() -> list[tuple[str, tuple[tuple[str, str], ...]]]:
	sequences: list[tuple[str, tuple[tuple[str, str], ...]]] = []
	seen = set()
	for line in SEQUENCES.read_text(encoding="utf-8").splitlines():
		line = line.strip()
		if not line or line.startswith("#"):
			continue
		parts = line.split("\t")
		tag = parts[0]
		if tag not in STEM_TAGS:
			continue
		sequence = []
		for item in parts[1:]:
			form, pos = item.split("/", 1)
			sequence.append((form, pos))
		key = (tag, tuple(sequence))
		if sequence and key not in seen:
			seen.add(key)
			sequences.append(key)
	return sequences


def stem_frequency(verb: str, frequencies: dict[str, float]) -> tuple[float, float] | None:
	if verb in frequencies:
		return frequencies[verb], 0.0
	for suffix, _ in DERIVATIONS:
		if verb.endswith(suffix):
			base = verb[: -len(suffix)]
			if base in frequencies:
				return frequencies[base], DERIVED_PENALTY
	return None


def stem_tag(kiwi: Kiwi, verb: str) -> tuple[str, str] | None:
	"""Return (sequence tag, join tag), or None for a non-verb homograph."""
	if verb == "이":
		return "VCP", "VCP"
	if verb == "아니":
		return "VCN", "VCN"
	tokens = kiwi.tokenize(verb + "다")
	if not tokens:
		return None
	first = tokens[0]
	base = first.tag.split("-")[0]
	if first.form == verb and base in STEM_TAGS:
		tag = "VV" if base == "VX" else base
		return tag, first.tag
	if first.tag in NOUN_TAGS:
		for token in tokens[1:]:
			base = token.tag.split("-")[0]
			if base == "XSV":
				return "VV", "VV"
			if base == "XSA":
				return "VA", token.tag
	return None


def score(probability: float, penalty: float) -> int:
	# The shipped dictionary stores hundredths of ln(frequency per million + 1).
	per_million = probability * 1_000_000
	return max(0, round((math.log(per_million + 1.0) - penalty) * 100))


def main() -> int:
	frequencies = wordfreq.get_frequency_dict("ko", "best")
	paradigm = Paradigm()
	kiwi = Kiwi()
	base = base_words()
	sequences = read_sequences()
	print(f"base forms: {len(base)}")
	print(f"ending sequences: {len(sequences)}")

	generated: dict[str, int] = {}
	stems = 0
	forms = 0
	for verb in paradigm.verb2verb_classes:
		tag_info = stem_tag(kiwi, verb)
		if tag_info is None:
			continue
		tag, join_tag = tag_info
		frequency = stem_frequency(verb, frequencies)
		if frequency is None:
			continue
		probability, derived_penalty = frequency
		penalty = GENERATED_PENALTY + derived_penalty
		stem_sequences = [
			sequence
			for sequence_tag, sequence in sequences
			if sequence_tag == tag
		]
		if tag == "VV" and verb.endswith("하"):
			stem_sequences.extend(tuple(sequence) for sequence in HA_SEQUENCES)
		if not stem_sequences:
			continue
		valid_stem = False
		for sequence in stem_sequences:
			if verb in {"있", "없"} and any(
				pos == "ETM" and form in {"은", "ᆫ"} for form, pos in sequence
			):
				# 있다/없다 take 는, not 은: 있는/없는, not 있은/없은.
				continue
			try:
				form = kiwi.join([(verb, join_tag)] + list(sequence))
			except Exception:
				continue
			if not form or " " in form or "/" in form or form in base:
				continue
			valid_stem = True
			forms += 1
			form_score = score(probability, penalty)
			old = generated.get(form)
			if old is None or form_score > old:
				generated[form] = form_score
		if valid_stem:
			stems += 1

	# `예요` is the vowel-final allomorph of `이에요`; Kiwi does not derive it
	# from `이` + `예요` without producing the non-standard `이예요`.
	if "이" in frequencies and "예요" not in base:
		generated["예요"] = max(
			generated.get("예요", 0),
			score(frequencies["이"], GENERATED_PENALTY),
		)

	lines = [
		f"{form}\t{form_score}\t{GENERATED_FLAG}"
		for form, form_score in sorted(generated.items())
	]
	OUT.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
	print(f"stems used: {stems}")
	print(f"generated forms: {len(generated)}")
	print(f"raw generated joins: {forms}")
	print(f"size: {OUT.stat().st_size} bytes")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
