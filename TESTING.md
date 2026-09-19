# Property tests & mutation harness

This document describes the fixed-seed property suite (`tests/properties.rs`
and `tests/properties/`) and the automated mutation entry
(`examples/prop_mutate.rs`).

## What the suite checks

The suite does **not** hard-code the statistical model into an answer table.
Ambiguous generated text asserts *invariants*; concrete language expectations
are only used where the public contract is unambiguous (single-language
scripts, single-candidate whitelists, Mandarin/Japanese percentage branches).

- **Determinism**: the same text and configuration always yields identical
  `(lang, script, confidence)` across repeated detections.
- **Confidence**: always finite and inside the public `0.0..=1.0` range.
- **Filters**: an allowlist result is always a member of the allowed set; a
  denylist never returns an excluded member. Same members in a different order
  agree. Empty candidate sets and one-member allowlists follow the documented
  current contract (see below).
- **Script stability**: pure Latin/Cyrillic/Arabic/Devanagari/Hebrew/Han
  fragments keep their script when punctuation, digits, `\n`/`\r\n`/`\r` and
  leading/trailing whitespace are added.
- **Mixed fragments**: Latin, Cyrillic, Arabic, Devanagari, Hebrew and
  Han/Kana pure and mixed fragments satisfy determinism and confidence bounds
  under all three supported `Method`s (`Trigram`, `Alphabet`, `Combined`).
- **Confusable profiles**: Spa/Por, Rus/Ukr and Ara/Pes real snippets record
  the top two languages and the confidence margin; hard assertions are
  limited to stability, legal candidates and ranking self-consistency —
  never "natural language X must always win".

There are 39 named `properties_*` tests.

## Current-public-contract notes

These are intentionally locked as the behavior the code ships today, not as
claims about statistical correctness:

- `Method::Alphabet` for Arabic/Devanagari/Hebrew uses an unranked mock:
  every surviving candidate scores `1.0`. With one allowed language the
  single-candidate rule (confidence `1.0`) applies; with several allowed
  languages the result is the mock's iteration order — the property tests
  only assert the candidate is legal.
- The Mandarin special branch (`detect_lang_base_on_mandarin_script`) only
  consults `is_allowed(Lang::Cmn)`:
  - a returned `Cmn` is always allowlisted and never denied;
  - the `Jpn` fallback is unconditional: denying `Jpn`, not allowlisting
    `Jpn`, denying `Cmn`, or using an empty allowlist can all yield `Jpn`
    once the percentage rule takes that path;
  - pure Hiragana/Katakana text takes the single-language `Jpn` branch, which
    also ignores filters.
- The percentage bands (kana fraction over Han+Kana) are
  `> 0.20 => Jpn 1.0`, `> 0.05 => Jpn 0.5`, `> 0.02 => Cmn 0.5`,
  otherwise `Cmn 1.0`. Boundary tests use exact fractions (e.g. `12/63 =
  0.190476...` lives strictly inside `(0.19, 0.20)`).

## Fixed seeds and replay

All generation goes through the dependency-free SplitMix64 RNG in
`tests/properties/common/mod.rs`, seeded from the fixed `SEEDS` table. Two
consecutive runs generate cases in the same order; this is itself asserted by
`properties_generator_sequence_is_replayable_in_order`.

To replay a single seed locally, set the `WHATLANG_PROP_SEED` environment
variable (parsed as `u64`) for the generator smoke test:

```sh
WHATLANG_PROP_SEED=12345 cargo test --test properties properties_generator_seed_override_env -- --nocapture
```

## Length and character sets

Generated fragments are 1–40 characters drawn only from restricted code point
ranges of the target script (`ScriptKind::ranges`), with ASCII spaces between
some words. Noise decoration uses ASCII punctuation/digits/line breaks/
whitespace only, all of which are detector stop characters. Mixed fragments
replace a bounded number of positions with a second script.

## Failure reports and shrinking

On failure the runner applies classic delta-debugging over code points
(`minimize`) and the assertion message contains:

- the seed and iteration;
- the minimized code point sequence (`U+....`);
- the filter configuration (`none`, `allow(...)`, `deny(...)`);
- the `Method`;
- the current candidate ranking (`winner`, `runner_up`, `margin`).

Shrinking only removes code points; it never changes the filter or method of
the failing case.

## Acceptance

Preparation (not part of the timed demo):

```sh
cargo build --all-targets
```

Acceptance, run from the repository root with no external services or
environment setup:

```sh
cargo test --quiet properties
```

The command exits zero and prints every named property test (the `properties`
filter matches the dedicated integration test binary `tests/properties.rs`).

## Mutation harness

Automated mutation entry:

```sh
cargo run --example prop_mutate
```

For each of three pre-defined, realistic defect classes it (1) verifies the
targeted property passes on unmutated code, (2) applies a single source
mutation, (3) requires a targeted property test to fail while naming the
broken invariant in its message, (4) restores the source immediately, (5)
verifies the targeted test passes again, and after all mutations (6) re-runs
the full properties suite and asserts the touched source files are clean via
`git status --porcelain`. A `Drop` guard restores the file even if the
harness panics.

| Mutation | File | Defect simulated | Caught by |
|---|---|---|---|
| `denylist_applied_after_ranking` | `src/trigrams/detection.rs` | profiles are ranked before the denylist is applied, so an excluded language can win | `properties_mut_target_denylist_applied_before_ranking` |
| `wrong_confidence_normalization_denominator` | `src/core/confidence.rs` | normalized confidence returns the raw unbounded margin rate instead of clamping to `1.0` | `properties_mut_target_confidence_normalization_range_trigram` |
| `script_threshold_boundary_off_by_one` | `src/core/detect.rs` | Mandarin/Japanese 20% threshold relaxed to 19% | `properties_special_branches_kana_threshold_confidence_levels` |

The harness only edits the three listed source files transiently and never
touches trigram profile data. It exits non-zero if any mutation survives, if
an invariant is not named, or if the working tree is left dirty.
