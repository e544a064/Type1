# Dictionary notice

`fr.tsv` is a derived list built from **Lexique 3.83**:

- Lexique 3.83, Boris New & Christophe Pallier — <http://www.lexique.org>
- Licence: Creative Commons Attribution-ShareAlike 4.0 International
  (CC BY-SA 4.0) — <https://creativecommons.org/licenses/by-sa/4.0/>
- Derivation: lowercase single tokens made of letters and inner apostrophes,
  with film and book frequencies summed per orthographic form; the 20 000 most
  frequent forms are kept. Each line is
  `word<TAB>ln(frequency per million + 1) × 100<TAB>pos-flags`, where the flags
  are 1 verb, 2 noun, 4 adjective and 8 adverb (OR-ed across entries).
- This derived data is distributed with the GPL-3.0 application under the same
  CC BY-SA 4.0 terms for the data itself.

`en.tsv` is a derived list built from **wordfreq**:

- wordfreq, Luminoso Technologies — <https://github.com/rspeer/wordfreq>
- Export used: <https://github.com/aparrish/wordfreq-en-25000>
- Licence: Creative Commons Attribution-ShareAlike 4.0 International
  (CC BY-SA 4.0) — <https://creativecommons.org/licenses/by-sa/4.0/>
- Derivation: single alphabetic tokens, top 20 000 forms by frequency; common
  American spellings are replaced by their British variants (`colour`, `centre`,
  `organise`, `favourite`), while words valid in both varieties
  (`meter`/`metre`, `program`/`programme`, `tire`/`tyre`) are kept. The pronoun
  `I` is kept capitalised. Each line is
  `word<TAB>ln(frequency per million + 1) × 100<TAB>0`.
- This derived data is distributed with the GPL-3.0 application under the same
  CC BY-SA 4.0 terms for the data itself.
