# Autocorrection: Algorithm Specification

This document specifies the correction and suggestion algorithm used by the
keyboard. It is implementation-independent: it describes the inputs, the cost
model, candidate generation, ranking, the automatic-correction decision, and
the editing behaviour that surrounds them.

All scores are additive and unitless. Every numeric threshold below is
calibrated to the frequency scale described in §2.

## 1. Pipeline

For a typed token `t`:

1. Normalise `t` to lower case and compute its accent-folded form `F(t)`.
2. If `t` contains an apostrophe, produce no suggestions and no correction.
3. If `t` has exactly one character, use the single-letter suggestion path.
4. Otherwise, if `t` has fewer than two characters or mixed capitalisation,
   produce no suggestions and no correction.
5. Look up whether `t` is already a known word.
6. Generate candidate words from the active language dictionaries.
7. Merge duplicates, order the candidates, and keep at most six.
8. Decide whether the best candidate is confident enough to replace `t`
   automatically.
9. During typing, show the ordered candidates in the suggestion strip.
   Automatic replacement happens only when a word boundary is committed
   (space, punctuation or an editor action), or when the user taps a
   suggestion.

Autocorrection can be disabled by the host for non-prose fields. When it is
disabled, the entire pipeline is skipped: no candidates and no replacement.

In one sentence: every multi-character candidate is a dictionary word or an
apostrophe-restored form of one; its score is `1.5 × log-frequency` minus a
layout-aware error cost; candidates are grouped into confidence tiers, and
auto-apply requires both an absolute confidence threshold and a margin over the
next-best candidate.

## 2. Inputs and terminology

### 2.1 Token

A token is a maximal run of alphabetic characters and apostrophes. Digits,
spaces and punctuation end a token. A typed apostrophe is part of the token, so
a token that already contains one is never corrected. Missing-apostrophe forms
are restored by the elision and contraction rules instead.

Let:

- `t` be the typed token in lower case;
- `F(x)` be the folded form of a string;
- `|F(x)|` be the number of symbols in its folded form.

### 2.2 Layout and key geometry

The layout supplies the ordered sequence of letter keys and, for each key, its
physical row: top, home, or bottom. The letters are on one visual line; a key
has:

- horizontal coordinate `x` equal to its position in that ordered sequence;
- vertical coordinate `y = 0.55 * row_index`, where the top, home and bottom
  rows have indices 0, 1 and 2.

The exact order is layout-specific. For example, a French AZERTY layout places
the first top-row letter, then the first home-row letter, then the first
bottom-row letter, then the next column, and so on. The geometry describes the
one-line presentation, not a conventional three-row keyboard grid.

### 2.3 Lexicon entry

Each entry carries:

- a surface form, which is the spelling inserted into the text;
- a folded form;
- a log-frequency score `L` (higher means more frequent);
- part-of-speech flags used by elision rules: verb, noun, adjective, adverb.

The reference lexicons use

`L = ln(frequency per million + 1)`.

All thresholds below assume this scale. If a different frequency measure is
used, the constants must be recalibrated.

The lexicon supports two indexes:

- exact surface-form lookup;
- folded-form lookup. When several surface forms share one folded form, the
  folded index keeps the highest-frequency surface form.

### 2.4 Profile

A profile combines one layout with one or more languages. Candidate generation
uses the union of all active dictionaries. Exact and folded lookups return the
highest-frequency match across the active languages.

Elision is enabled if any active language enables it. Contraction tables belong
to individual languages, with a cross-language conflict rule described in
§7.

## 3. Normalisation

### 3.1 Accent folding

Folding lowercases the input and removes common Latin diacritics:

- `œ → oe`, `æ → ae`;
- `à â ä á → a`;
- `é è ê ë → e`;
- `î ï í → i`;
- `ô ö ó → o`;
- `ù û ü ú → u`;
- `ç → c`, `ÿ → y`, `ñ → n`.

Other characters are retained in lower case. Apostrophes are retained.

Examples: `École → ecole`, `où → ou`, `œuf → oeuf`.

The weighted distances operate on folded forms. Folding means that an accent
difference has zero substitution cost; a dedicated missing-accent channel gives
it a small positive explanation cost instead.

### 3.2 Capitalisation

Only alphabetic characters participate in the case pattern:

- **Lower**: no uppercase letters.
- **Capitalized**: the first alphabetic character is uppercase and no other
  alphabetic character is uppercase.
- **All caps**: at least one uppercase letter and no lowercase letters.
- **Mixed**: any other combination.

Mixed-case tokens are not corrected and do not receive suggestions. This
protects names, camel-case identifiers and other deliberate capitalisation.

Candidate surface forms are re-cased when they are shown or applied:

- **Lower**: keep the dictionary surface form as it is.
- **Capitalized**: upper-case the first character of the candidate.
- **All caps**: upper-case the entire candidate.

The candidate itself is never stored with the user's casing; the user's case is
applied only after ranking, when the candidate is displayed or inserted.

## 4. Explanation cost

### 4.1 Substitution cost

For two folded symbols `a` and `b`, the substitution cost is:

```
s(a, b) =
  0                                  if a == b,
  min(2.6, sqrt(
      (x(a) - x(b))^2
      + (y(a) - y(b))^2
  ))                                 if both are letters a..z,
  0                                  if both are non-letters,
  1.5                                if exactly one is a letter.
```

All non-letter symbols share one placeholder, so two different non-letters are
interchangeable at zero cost. Replacing a letter with any non-letter costs 1.5.

Because the vertical offset is `0.55` per row, adjacent positions in the
ordered line cost roughly `1.1` to `1.5`; positions two apart cost roughly `2.0`
to `2.3`; farther pairs approach the cap of `2.6`.

### 4.2 General weighted distance `D(a, b; limit)`

`D` is an optimal-string-alignment variant of Damerau–Levenshtein distance over
folded symbols:

- deletion or insertion: cost `1.0`;
- substitution: cost `s(a, b)`;
- adjacent transposition: cost `0.6`;
- overlapping edits are not combined: a transposition consumes two positions
  and adds `0.6` to the cost two positions back.

The distance is rejected if:

- the length difference exceeds 2;
- either string exceeds 31 symbols;
- the minimum possible remaining cost in a dynamic-programming row already
  exceeds the request limit.

If the final cost is at most the limit, that cost is returned; otherwise the
candidate is rejected.

### 4.3 Same-length keyboard distance `N(a, b; limit)`

`N` is used by the whole-word neighbour rescue. It requires equal length and
aligns the two strings without insertions or deletions:

- at position `i`, a substitution is allowed only if
  `s(a[i], b[i]) <= limit`; its cost is added to the prefix cost;
- an adjacent transposition is allowed at cost `0.6` and consumes two
  positions;
- the result is the sum of the accepted costs.

A string pair may have a high total cost, but every individual substitution
must be inside the limit. This is what allows a nonsense token to point at a
word when every key is one or two keys away.

Two limits are used:

- `1.5` for the one-key ring;
- `2.3` for the two-key ring.

### 4.4 One-letter spelling variant

A spelling variant is considered only when:

- the folded strings have equal length;
- the length is at least 4;
- exactly one position differs.

Its cost is fixed at `4.0`. This catches variants that are not close on the
keyboard, for example `commant → comment`, without allowing arbitrary distant
words.

## 5. Candidate channels

Candidate channels are not applied in final priority order. They produce
candidates into one pool; the pool is then ranked by the tier and score rules
in §6. The channels are described here in the order they are usually applied.

If the typed token is already a known word, the direct-edit, missing-accent and
elision channels are skipped; only the known baseline, whole-word neighbour
rescue and contractions can produce candidates. If the token is unknown, every
channel described below is eligible.

Each candidate also carries a confidence tier: tier 1 for trusted explanations,
tier 2 for weaker ones, and tier 3 for the last-resort fallback. The tier is
part of the final ranking; the exact membership of each tier is summarised in
§6.3.

### 5.1 Known-word baseline

Let `known` be the exact lower-case lookup of `t` in the active dictionaries.
If several languages contain it, keep the entry with the highest `L`. A form
that exists only with canonical capitalisation is not an exact known word,
although it can still be produced by the candidate channels.

If `known` exists, insert it as a candidate:

- tier 1;
- score `1.5 * L(known)`.

The known word's score is also remembered as the baseline for the
known-word margin in §8.2.

The baseline is not a bias that always wins. A sufficiently more likely
candidate can displace it.

### 5.2 Missing accents

If the folded typed form exactly equals a dictionary word's folded form, add
that word as:

- tier 1;
- score `1.5 * L(word) - 0.25`.

Example: `ecole → école`, `oeuf → œuf`.

This channel is skipped when `t` is already an exact known word.

### 5.3 Direct weighted-edit candidates

When `t` is not an exact known word, scan all dictionary words with at least two
folded symbols. For each word `w`:

- compute `D(F(t), F(w); 1.55)`;
- reject zero-distance matches (these are the missing-accent case);
- let `delta = abs(|F(w)| - |F(t)|)`;
- insert a tier-1 candidate with score

```
1.5 * L(w) - D - 2.0 * delta
```

The `2.0 * delta` term makes inserting or deleting characters less likely than
pressing a neighbouring key. The direct channel therefore usually represents a
single substitution, transposition, or one insertion/deletion. Multiple edits
are possible only if their total weighted cost remains at most `1.55`.

### 5.4 French elision

Elision restores a missing apostrophe between a clitic and a following word.
This channel is active only for languages that enable elision, and like the
direct channel it is generated only when the token is not already an exact known
word.

#### Clitics and penalties

| Clitic | Penalty |
|---|---:|
| `jusqu`, `lorsqu`, `puisqu`, `presqu` | 0.4 |
| `qu` | 0.7 |
| `j`, `t`, `l`, `d`, `n`, `s`, `m`, `c` | 0.6 |

The typed lower-case token must start with the clitic. The remainder must have
at least one character for clitics of three or more characters, and at least
two characters for the shorter clitics.

#### Exact remainder

If the folded remainder is exactly a dictionary word and the elision is
allowed, insert:

- surface form `clitic + "'" + remainder`;
- tier 1;
- score

```
1.5 * L(remainder) - clitic_penalty - 0.15
```

The `0.15` is the apostrophe cost.

Special case: if the clitic is `jusqu` and the remainder is the ordinary word
`a`, use the accented word `à` as the remainder when it exists, so the candidate
is `jusqu'à`, not `jusqu'a`.

#### Fuzzy remainder

If the folded remainder has at least three symbols, has the same length as a
dictionary word, and is a weighted edit distance of at most `1.55` away, then
for every such word that passes the elision rules add:

- tier 1;
- score

```
1.5 * L(remainder) - clitic_penalty - 0.15 - D(remainder, word)
```

with the distance greater than zero. This is how `taimx → t'aime` is reached.

#### Elision eligibility

The remainder must begin with a vowel, or with `h` except after `qu`:

- vowels: `a e i o u y à â ä é è ê ë î ï ô ö ù û ü œ æ`;
- `qu` requires a vowel; it does not accept `h`.

Additional part-of-speech rules:

- `j`, `t`, `m`, `n`: the word must be a verb, or be exactly `en` or `y`;
  `t` is not allowed before `est`.
- `s`: the word must be a verb, or be exactly `il`, `ils`, `on`, `y` or `en`.
- `c`: the word must be `est`, `était`, `étaient`, `eût`, `en`, or start with
  `est-`.
- `d`: the word must be a noun or adverb, or an infinitive verb, or be one of
  `où`, `ici`, `ailleurs`, `abord`, `après`, `avant`, `ores`.
- `l`: the word must be a noun, verb or adjective, or be exactly `on`.
- `jusqu`, `lorsqu`, `puisqu`, `presqu`, `qu`: only the initial-vowel rule
  applies (`qu` uses the stricter vowel-only rule).

An infinitive is a word ending in `er`, `ir`, `re` or `oir`.

### 5.5 Whole-word neighbour rescue

This channel applies even when `t` is already a known word. For every dictionary
word with the same folded length as `t`:

1. Try the same-length keyboard distance `N` with limit `1.5`.
   - If it succeeds, the candidate is tier 1 with cost `N`.
2. Otherwise try `N` with limit `2.3`.
   - If it succeeds, the candidate is tier 2 with cost `N`.
3. Otherwise try the one-letter spelling variant.
   - If it succeeds, the candidate is tier 2 with cost `4.0`.

The score is:

```
1.5 * L(word) - cost
```

This channel is deliberately aggressive: it can identify a common word when
every typed letter is one or two keys away, as in `geqnvx → beauté`. Tier 2
candidates are shown but require a higher confidence before automatic
replacement.

### 5.6 Aggressive elision

This channel handles a typo in the clitic as well as in the remainder. It is
applied only when:

- `t` is not an exact known word;
- the typed token has at least three characters;
- the language enables elision.

For each clitic and each language:

1. Compare the first `len(clitic)` symbols of the typed token with the clitic
   using the keyboard distance `N`; both tiers are accepted.
2. Require enough remainder characters as in the exact-elision rule.
3. For every dictionary word with the same folded length as the remainder and
   an allowed elision, compute the remainder's keyboard distance.
4. Let

```
cost = prefix_cost + remainder_cost + 0.15
tier = max(prefix_tier, remainder_tier)
```

and insert:

```
surface: clitic + "'" + remainder
score:   1.5 * L(remainder) - clitic_penalty - cost
```

Example: `vqjmd → t'aime`.

### 5.7 Curated contractions

Each language may supply a finite mapping from apostrophe-less keys to
canonical contractions, for example `dont → don't`, `im → I'm`,
`oclock → o'clock`. The mapping is language data, not learned.

Before applying a language's contraction table, check the other active
languages. If another active language has an exact dictionary word equal to the
typed lower-case form, skip this language's contractions entirely. This prevents
an English contraction from overriding a valid word in a mixed-language
profile.

Two cases are applied:

- **Exact key**: if `t` equals a contraction key, insert the contraction with
  tier 1 and score

```
8.0 * 1.5 = 12.0
```

- **Alias**: if any already-generated candidate has the contraction key as its
  surface text, remove that candidate and re-insert the contraction with the
  same tier and score

```
candidate_score + 4.0
```

The alias rule means a typo whose best ordinary candidate is `dont` can still
produce `don't`.

Some keys are also ordinary words. A language lists these as ambiguous
contractions (`lets` in English). They still produce the contraction candidate
and suggestion, but the engine only auto-applies the contraction when the word
started at a sentence boundary. Mid-sentence `lets` is left as typed while
`let's` remains available in the strip.

A contraction's surface form may contain an apostrophe and may have canonical
capitalisation. Case is applied to it like any other candidate.

### 5.8 Last resort

If no candidate exists after all other channels, use the general weighted
distance as a fallback. It is not used when even one normal, neighbour,
elision or contraction candidate exists.

Conditions:

- `|F(t)| >= 4`;
- only words with the same folded length as `t`;
- compute `D` with a limit of `|F(t)| * 2.6`;
- retain only words whose distance is within `0.5` of the best distance.

Order these words by:

1. smaller distance;
2. higher log-frequency;
3. lexicographically smaller surface form.

They receive tier 3 and score

```
-distance + 0.001 * L(word)
```

Tier 3 candidates are shown but are never applied automatically.

### 5.9 Single-letter suggestions

A token of exactly one character uses a separate path and never triggers
automatic correction.

1. Fold the character. It must become one ASCII letter `a..z`; otherwise return
   no suggestions.
2. Build a neighbour list from every other letter whose substitution cost is
   greater than zero and at most `2.3`, ordered by increasing cost and then by
   letter. If the list is empty, use the globally closest other letter.
3. The allowed base set is the typed base plus all listed neighbours.
4. Collect every one-letter dictionary word whose folded base is allowed. For
   each folded base, keep the highest-frequency surface form of each distinct
   text.
5. Discard the plain folded base text and the exact typed text.
6. If the typed form differs from its plain folded base and that base is itself a
   word (for example, typing `à` and finding `a`), add the base.
7. Sort these word candidates by score `1.5 * L` descending, then by text.
8. Append the raw neighbour letters that were not already emitted, as tier 2
   with score `-cost`.
9. Keep at most six candidates.

The result is therefore one-letter words and accented variants first, followed
by the keys that were probably intended. No single-letter candidate is applied
automatically.

## 6. Merging and ranking

### 6.1 Duplicate handling

Candidates are keyed by their full surface text. When two channels produce the
same surface text:

- keep the lower tier;
- if the tiers are equal, keep the higher score.

This means a confident direct candidate is not duplicated by a weaker spelling
variant, and a contraction alias replaces the ordinary candidate rather than
appearing beside it.

### 6.2 Final order

Before sorting, if at least one tier-2 candidate exists, discard any tier-1
candidate other than the exact known-word baseline whose score is more than
`5.5` below the best tier-2 score. This prevents a rare word that happens to be
one key away from crowding out a much more likely spelling variant. The
known-word baseline is exempt so a valid rare word is not displaced by a merely
common spelling variant.

Sort all retained candidates by:

1. tier ascending (1 before 2 before 3);
2. score descending;
3. surface text ascending.

Keep at most six candidates.

This sort applies to the normal candidate pool. The last-resort list is the
exception: it is already ordered by distance, then frequency, then text, and is
not re-sorted by score.

If the typed token is an exact known word and that known word is the best
candidate, the suggestion list is empty: the corrector has nothing to offer.
If the known word is not the best, remove the known surface form from the list
before truncation, so the strip does not repeat the typed word as a “correction”.

### 6.3 What each tier means

| Tier | Meaning |
|---|---|
| 1 | Trusted explanation: known baseline, missing accent, direct weighted edit, elision, exact contraction, or a neighbour word whose substitutions are all in the one-key ring. |
| 2 | Weaker explanation: at least one substitution in the outer key ring, or a one-letter spelling variant. |
| 3 | Last-resort same-length edit-distance match. Never auto-applied. |

The tier is a confidence class, not a channel order. Within tier 1, a very
frequent neighbour can outrank a rare direct edit.

## 7. Multi-language behaviour

The active dictionaries are pooled:

- exact lookup chooses the highest-frequency exact surface form across
  languages;
- folded lookup chooses the highest-frequency folded match across languages;
- elision is enabled if any active language enables it;
- contractions are considered language by language.

The contraction conflict rule in §5.7 prevents a contraction key from
overriding a valid word in another active language.

## 8. Automatic correction

### 8.1 When correction may run

Autocorrection is considered only at a word boundary:

- space;
- comma, period or any other word-ending punctuation/symbol;
- the enter key;
- an editor action that ends the word.

While the user is still typing, only suggestions are shown. The user can tap
one to replace the word immediately.

The following tokens are never corrected:

- tokens containing an apostrophe;
- tokens shorter than two characters;
- mixed-case tokens;
- tokens for which correction is disabled by the host;
- tokens marked as already reverted by an undo.

The best candidate is not applied if it is identical to the typed token.

### 8.2 Confidence rules

Let `best` be the first candidate after ranking.

1. **Tier threshold**:
   - tier 1: apply only if `best.score >= 0.5`;
   - tier 2: apply only if `best.score >= 2.0`;
   - tier 3: never apply.
2. **Known-word margin**: if the typed token is a known word with baseline score
   `1.5 * L(known)`, require

   ```
   best.score - 1.5 * L(known) >= 5.5
   ```

   This protects common words and, with a large lexicon, rare but valid words:
   `la` stays `la`, `radiologie` stays `radiologie`, while a much more likely
   neighbour can still replace a rare word such as `pa → la`.
3. **Runner-up margin**: if another candidate has the same tier as `best`,
   require

   ```
   best.score - runner_up.score >= 0.25
   ```

   Otherwise the correction is ambiguous and is not applied automatically. A
   lower-tier candidate does not block the best candidate.

The tier thresholds and both margins operate on the same score scale. The
known-word margin is the likelihood ratio between the typed word and the
replacement in log-score space.

### 8.3 Applying the correction

When all conditions pass, the engine:

1. deletes the typed token;
2. inserts the corrected word with the candidate's case applied;
3. commits the word boundary or performs the editor action that triggered the
   correction;
4. for a normal word boundary, remembers the original typed token, the corrected
   form, the boundary text, and up to two alternative candidates for undo and
   for the post-correction suggestion strip. An editor action applies the
   correction but does not keep a boundary-undo record.

If no candidate passes, the typed token is left untouched and the boundary or
editor action proceeds normally.

## 9. Suggestion strip and editing behaviour

### 9.1 Strip size

The number of visible suggestions depends on the length of the source token:

| Source length | Maximum suggestions |
|---|---:|
| 0–3 | 6 |
| 4 | 5 |
| 5 | 4 |
| 6 or more | 3 |

The corrector itself produces at most six. The strip applies the tighter
length-based limit after case application; the single-letter and existing-text
paths also suppress duplicate display forms.

### 9.2 While typing

For the current word, the strip:

- uses the current word plus any known suffix;
- applies the typed word's case pattern to each candidate;
- suppresses candidates equal to the typed text and, on the paths that require
  it, duplicate display forms;
- keeps the first few according to the table above.

For a known word that is already the best explanation, the strip is empty.

### 9.3 After an automatic correction

Immediately after a correction, the strip shows:

1. the original typed form first, so tapping it undoes the correction;
2. up to two alternative corrections, excluding the corrected word and the
   original.

This list is retained until the next word boundary or until the user edits the
word again.

### 9.4 Undo

A backspace immediately after a boundary correction:

- deletes the corrected word and its boundary;
- restores the original typed word and the same boundary;
- leaves the cursor before the boundary so the boundary stays in the text;
- marks the word as reverted so it is not corrected again until the user edits
  it.

The next boundary reuses or steps over the preserved boundary instead of
duplicating it. Typing a letter clears the reverted guard, so a later boundary
can correct the newly edited word normally. Moving the caret or typing a letter
also discards the pending correction record, so a later backspace no longer
undoes the correction.

### 9.5 Selecting a suggestion

Tapping a suggestion:

- replaces the current typed word with the chosen candidate;
- for a normal current word, adds the language's convenience space after the
  word; when the suggestion replaces existing text around the caret, it replaces
  only that word and does not add a space;
- records the change so that a later caret move can recover the original typed
  form.

If the user deletes the convenience space immediately, the accepted word
becomes the active word again and is protected from a second automatic
correction until it is edited.

If the suggestion is selected after a correction, the existing boundary is
preserved and the correction record is updated. Selecting the original typed
form cancels the correction.

### 9.6 Re-correcting existing text

When the caret moves into an existing word, the engine reads the maximal run of
word characters before and after the caret. If the combined word has at least
two characters and is not mixed case, it generates suggestions for the whole
word.

If the visible word was previously produced by a correction, the engine uses a
per-field correction memory to recover the form the user originally typed and
generates candidates from that form. Candidates equal to either the visible
word or the original typed form are removed. Tapping a candidate replaces the
whole surrounding word, not just the part before or after the caret.

The correction memory:

- stores only words the keyboard actually changed;
- maps the lower-case visible word to the typed form;
- is newest-first and bounded to 1000 entries or 256 KiB;
- is reset when the input field changes;
- is not consulted while a new word is being typed normally.

## 10. Parameter summary

| Parameter | Value |
|---|---:|
| Frequency weight | 1.5 |
| Layout row offset | 0.55 per row |
| Substitution cost cap | 2.6 |
| Letter/non-letter substitution cost | 1.5 |
| Normal direct-distance limit | 1.55 |
| One-key distance limit | 1.5 |
| Two-key distance limit | 2.3 |
| Length-change penalty | 2.0 per symbol |
| Transposition cost | 0.6 |
| Apostrophe cost | 0.15 |
| Missing-accent cost | 0.25 |
| One-letter spelling-variant cost | 4.0 |
| Tier-1 minimum score | 0.5 |
| Tier-2 minimum score | 2.0 |
| Known-word margin | 5.5 |
| Tier-1 fallback margin | 5.5 |
| Same-tier runner-up margin | 0.25 |
| Maximum token length for weighted distance | 31 symbols |
| Last-resort minimum length | 4 symbols |
| Last-resort tie band | 0.5 |
| Maximum candidates generated | 6 |
| Contraction exact log-frequency | 8.0 |
| Contraction alias bonus | 4.0 |
| Correction memory entries | 1000 |
| Correction memory bytes | 256 KiB |

## 11. Worked examples

These examples use the reference French and English lexicons and the reference
layouts. Unless stated otherwise, each example uses the language in which the
result is valid. Exact output can change if the lexicons or layouts change.

| Typed | Result | Explanation |
|---|---|---|
| `dkre` | `dire` | one-key substitution |
| `tset` | `test` | adjacent transposition, cost 0.6 |
| `ecole` | `école` | missing-accent candidate, cost 0.25 |
| `jai` | `j'ai` | exact elision, clitic penalty |
| `taimx` | `t'aime` | fuzzy elision of the remainder |
| `vqjmd` | `t'aime` | aggressive elision with a clitic typo |
| `dont` | `don't` | exact English contraction |
| `pa` | `la` | rare known word displaced by a much more frequent neighbour |
| `la` | `la` | common known word stays |
| `radiologie` | `radiologie` | rare known word stays |
| `geqnvx` | `beauté` | whole-word neighbour rescue despite many key slips |
| `commant` | `comment` | one-letter spelling variant, tier 2 |
| `xqzv` | no auto-correction | nothing passes the strong channels; any fallback is tier 3 |
| single `q` | `à`, `a`, `w`, `z` | one-letter words first, then neighbouring keys |

## 12. Invariants and edge cases

- A token with a typed apostrophe is never corrected; apostrophe restoration is
  performed only by elision and contraction rules.
- A token with mixed capitalisation is never corrected or suggested.
- A token of one character uses only the single-letter path and is never
  auto-corrected.
- A token longer than 31 folded symbols is not considered by the weighted
  distance.
- The general weighted distance rejects length differences greater than two.
- The direct channel requires a non-zero distance; folded-exact matches are
  handled by the missing-accent channel.
- The whole-word neighbour channel requires equal length; length-changing edits
  come only from the direct, elision or contraction channels.
- The last-resort channel is used only when no other channel produced a
  candidate.
- Tier-3 candidates are never auto-applied.
- A one-key candidate more than `5.5` score units below the best tier-2
  candidate is discarded, except for the exact known-word baseline.
- A known word that is already the best candidate produces an empty suggestion
  list.
- Duplicate surface forms keep the lower tier, and then the higher score.
- Final ordering is deterministic: tier, then score, then text for the normal
  pool; the last-resort pool keeps its distance-first order.
