# Dictionary notice

`fr.tsv` is a derived list built from **Lexique 3.83**:

- Lexique 3.83, Boris New & Christophe Pallier — <http://www.lexique.org>
- Licence: Creative Commons Attribution-ShareAlike 4.0 International
  (CC BY-SA 4.0) — <https://creativecommons.org/licenses/by-sa/4.0/>
- Derivation: lowercase single tokens made of letters and inner apostrophes,
  with film and book frequencies summed per orthographic form. Spaces, hyphens
  and other non-token punctuation are excluded because the keyboard tokenizer
  cannot use them; all remaining forms with positive frequency and at most 31
  characters are kept (about 121,000 entries). Each line is
  `word<TAB>ln(frequency per million + 1) × 100<TAB>pos-flags`, where the flags
  are 1 verb, 2 noun, 4 adjective and 8 adverb (OR-ed across entries).
- This derived data is distributed with the GPL-3.0 application under the same
  CC BY-SA 4.0 terms for the data itself.

`en.tsv` is a derived list built from **wordfreq**:

- wordfreq, Luminoso Technologies — <https://github.com/rspeer/wordfreq>
- Export used: the full English `best` list from the wordfreq package
- Licence: Creative Commons Attribution-ShareAlike 4.0 International
  (CC BY-SA 4.0) — <https://creativecommons.org/licenses/by-sa/4.0/>
- Derivation: every single alphabetic token up to 31 characters with a positive
  wordfreq probability (about 287,000 forms, 3.6 MB), including names, places,
  brands and modern terms; common American spellings are replaced by their
  British variants (`colour`, `centre`, `organise`, `favourite`), while words
  valid in both varieties (`meter`/`metre`, `program`/`programme`,
  `tire`/`tyre`) are kept. The pronoun `I` is kept capitalised. Each line is
  `word<TAB>ln(frequency per million + 1) × 100<TAB>0`.
- This derived data is distributed with the GPL-3.0 application under the same
  CC BY-SA 4.0 terms for the data itself.

`es.tsv` is a derived list built from **wordfreq**:

- wordfreq, Luminoso Technologies — <https://github.com/rspeer/wordfreq>
- Export used: the full Spanish `best` list from the wordfreq package
- Licence: Creative Commons Attribution-ShareAlike 4.0 International
  (CC BY-SA 4.0) — <https://creativecommons.org/licenses/by-sa/4.0/>
- Derivation: every single token up to 31 characters made of Spanish letters
  (including `á é í ó ú ü ñ`) with a positive wordfreq probability (about
  334,000 forms, 4.5 MB), including names, places and modern terms. Each line is
  `word<TAB>ln(frequency per million + 1) × 100<TAB>0`.
- This derived data is distributed with the GPL-3.0 application under the same
  CC BY-SA 4.0 terms for the data itself.

`ko_conj.tsv` contains generated Korean conjugation forms:

- Generator: **KoParadigm 0.10.0**, Apache License 2.0,
  <https://github.com/Kyubyong/KoParadigm>; its verb/ending inventory derives
  from the Standard Korean Language Dictionary (CC BY-SA 2.0 KR),
  <https://stdict.korean.go.kr/>.
- Surface-form joining and irregular allomorphs: **Kiwi 0.25.1**, Apache License
  2.0, <https://github.com/bab2min/Kiwi>. The Kiwi model files are also
  Apache-2.0 and were trained on the 21st Century Sejong Corpus and the National
  Institute of Korean Language "모두의 말뭉치" corpora; those source corpora are
  not redistributed here.
- Build: `scripts/build-korean-conjugations.py` using
  `scripts/korean_sequences.tsv`. Forms already present in `ko.tsv` are omitted.
  Each generated form inherits its stem's wordfreq prior minus a small penalty
  and is marked with the `GENERATED` flag. Generated candidates are kept out of
  the attested-candidate filtering; at most the best one is promoted into the
  final suggestion list. A generated candidate can auto-apply for an unknown
  word, but never replaces a word the dictionary already knows.
- The generated data is distributed under CC BY-SA 4.0 with the rest of the
  Korean dictionary data. See `app/src/main/res/raw/license_korean_conjugations.txt`
  for the in-app notice.

- wordfreq, Luminoso Technologies — <https://github.com/rspeer/wordfreq>
- Export used: the full Korean `best` list from the wordfreq package
- Licence: Creative Commons Attribution-ShareAlike 4.0 International
  (CC BY-SA 4.0) — <https://creativecommons.org/licenses/by-sa/4.0/>
- Derivation: every single token up to 31 precomposed Hangul syllables
  (U+AC00–U+D7A3) with a positive wordfreq probability (about 26,800 forms,
  0.37 MB). The keyboard composes jamo into these syllables before lookup.
  Each line is `word<TAB>ln(frequency per million + 1) × 100<TAB>0`.
- This derived data is distributed with the GPL-3.0 application under the same
  CC BY-SA 4.0 terms for the data itself.
