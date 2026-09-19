# Testing

## Acceptance commands

Run both commands from the repository root; no external services, extra
environment variables or network access are required.

```sh
# Preparation (not part of the demo)
cargo build --all-targets

# Acceptance
cargo test --quiet properties
```

`cargo test --quiet properties` prints every named property case
(`properties::properties_00` … `properties::properties_34`) and the validation
stages, and exits with code `0` on success.

`cargo test --quiet` puts the built-in libtest reporter into terse mode (dots
only). The `property_harness` test target
(`tests/property_harness.rs`, `harness = false`) locates the freshly built
crate unit-test binary in the same `deps` directory and re-runs the
`properties` filter with the pretty reporter, propagating the child exit code.
It is test-only infrastructure: it does not change any production API.

## Property suite layout

The property tests live in `src/properties.rs` (an internal `#[cfg(test)]`
module, so they can use `pub(crate)` types such as `Options`, `Method`,
`InternalQuery` and `Text` without opening any production backdoor).

Coverage stages:

- **seeded-generators** — fixed-seed SplitMix64 RNG; pure fragments for Latin,
  Cyrillic, Arabic, Devanagari, Hebrew, Han, Hiragana and Katakana pools, plus
  mixed-script fragments with a deterministic dominant part.
- **determinism** — repeated detection of identical input + configuration is
  bit-for-bit equal, for all three public `Method`s.
- **confidence-range** — `Info::confidence()` is always finite and inside the
  public `0.0..=1.0` range; tied-score edge cases of
  `calculate_confidence` are checked directly.
- **whitelist-blacklist-contract** — allowlisted results come only from the
  allowed set; denylisted languages never survive into the candidate ranking.
  The Mandarin/Japanese special branch has explicit, separately named
  assertions because it follows its own documented contract.
- **empty-and-single-candidate** — denying every language of a multi-language
  script returns `None`; a one-language allowlist returns that language with
  confidence `1.0`.
- **filter-matrix-and-ordering** — allow/deny lists of several sizes, same
  members in different orders (forward/reversed/shuffled), mutually exclusive
  configurations.
- **mandarin-japanese-special-branch** — allowlist/denylist fallbacks and the
  exact `jpn_pct` boundary values (`0.20`, `0.05`, `0.02`; strict inequalities).
- **script-invariance-under-decoration** — pure single-script fragments keep
  their detected script after adding ASCII punctuation, digits, line breaks
  (`\n`, `\r`, tab, vertical tab) and leading/trailing whitespace.
- **mixed-scripts** — seeded mixed fragments keep their dominant script.
- **confusable-language-pairs** — Spa/Por (Latin, Combined), Rus/Ukr
  (Cyrillic, Combined), Ara/Pes (Arabic, Trigram). For each pair the top two
  candidates, their scores and the margin are recorded; assertions only cover
  stability of identical input + config, candidate-set legality vs. the filter,
  and descending score order. The suite never asserts that generated text must
  classify as one fixed natural language.
- **all-public-methods** — `Method::Trigram`, `Method::Alphabet` and
  `Method::Combined` are all exercised.

There are at least 24 named tests (currently 35, including the inventory
test). Samples whose concrete language is statistically unsuitable to lock
down only assert invariants; no hard-coded statistical answer table is used.

## Seed replay

Every generator is driven by `Rng::new(seed)` (SplitMix64) with hard-coded
seed constants per test. To reproduce a sample, construct the same `Rng` with
the seed printed in the failure report and request the same family/length.
`properties_02_generation_order_matches_frozen_sequence` is a golden snapshot
of the first generated sequence; it fails if the generator changes, which
would invalidate replay.

Failure reports contain:

- the `seed` and sample index,
- the minimized codepoint sequence (`U+XXXX` notation),
- the filter configuration (`All` / `Allow(...)` / `Deny(...)`),
- the `Method`,
- the candidate ranking (top five `Lang=score`),
- the violated invariant and a detail line.

Two runs with the same seed generate samples in the same order.

## Sample shrinking

When an invariant fails on a generated sample, `minimize()` in
`src/properties.rs` applies delta debugging: it repeatedly removes chunks
(halving the chunk size) and then tries removing each codepoint individually,
keeping the shortest sequence for which the predicate still fails. The
minimized sequence is part of the failure report and replays with the same
seed and configuration.

## Mutation entry point

```sh
./scripts/mutate_check.sh
```

The script injects three defect classes one at a time, expects the named
property tests to fail while naming the broken invariant, then restores the
mutated production files with `git checkout` (also via an `EXIT` trap) and
checks the workspace is clean:

1. `m1_blacklist_after_ranking` — drops the pre-ranking denylist filter in
   `src/trigrams/detection.rs` (filtering applied only after ranking); caught
   by `properties_11_blacklist_never_returns_excluded_languages`.
2. `m2_confidence_denominator` — divides by the score gap instead of the
   second score in `src/core/confidence.rs`, producing NaN on tied scores;
   caught by
   `properties_09_calculate_confidence_helper_is_finite_and_bounded_on_tied_scores`.
3. `m3_script_threshold_offbyone` — turns the strict `jpn_pct > 0.05`
   boundary into `>=` in `src/core/detect.rs`; caught by
   `properties_20_mandarin_japanese_threshold_boundaries_are_exact`.

The script never touches profile data (`src/trigrams/profiles.rs`), does not
weaken accuracy fixtures, does not retry flaky cases, and adds no production
test-only hooks.
