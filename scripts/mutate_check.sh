#!/usr/bin/env bash
# Automated mutation entry for the property suite.
#
# Injects three classes of defects, one at a time, and verifies that the
# property tests catch each one and name the broken invariant:
#   m1_blacklist_after_ranking  - denylist applied only after ranking
#   m2_confidence_denominator   - wrong confidence normalization denominator
#   m3_script_threshold_offbyone - Mandarin/Japanese script boundary off-by-one
#
# Source files are restored after every mutation (and on exit/error), and the
# workspace is checked to be clean before finishing. Profile data is never
# touched; only the three production files listed below are mutated.
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TRIGRAM_SRC="src/trigrams/detection.rs"
CONFIDENCE_SRC="src/core/confidence.rs"
DETECT_SRC="src/core/detect.rs"
MUTATED_FILES=("$TRIGRAM_SRC" "$CONFIDENCE_SRC" "$DETECT_SRC")

restore_sources() {
    for file in "${MUTATED_FILES[@]}"; do
        git checkout -- "$file"
    done
}

clean_workspace() {
    if [[ -n "$(git status --porcelain -- "${MUTATED_FILES[@]}")" ]]; then
        echo "ERROR: mutated production files are still modified after restore" >&2
        git status --short >&2
        return 1
    fi
}

trap 'restore_sources' EXIT

restore_sources
clean_workspace || { echo "pre-flight failed: production sources must be clean" >&2; exit 1; }

run_expect_failure() {
    local mutation_name="$1"
    local test_filter="$2"
    local expected_keyword="$3"

    echo "== mutation: $mutation_name =="
    local output
    if ! output=$(cargo test --quiet "$test_filter" 2>&1); then
        if grep -Fq "$expected_keyword" <<<"$output"; then
            echo "CAUGHT: property suite detected '$expected_keyword'"
            return 0
        fi
        echo "ERROR: $mutation_name failed the build/tests, but the broken invariant was not named" >&2
        echo "$output" >&2
        return 1
    fi
    echo "ERROR: $mutation_name was NOT caught by property tests ($test_filter)" >&2
    return 1
}

# --- M1: blacklist is applied only after candidates have been ranked --------
inject_m1() {
    python3 - "$TRIGRAM_SRC" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
text = path.read_text()
old = """    for &(lang, lang_trigrams) in lang_profile_list {
        if !filter_list.is_allowed(lang) {
            continue;
        }
"""
new = """    for &(lang, lang_trigrams) in lang_profile_list {
        // MUTATION m1: filtering deferred until after ranking
"""
assert old in text, "m1 anchor not found"
path.write_text(text.replace(old, new, 1))
PY
}

# --- M2: confidence normalization denominator is wrong.
# Normalizing by the score gap instead of the second score divides by zero
# (NaN) whenever the two best candidates tie.
inject_m2() {
    python3 - "$CONFIDENCE_SRC" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
text = path.read_text()
old = "    let rate = (highest_score - second_score) / second_score;\n"
new = "    let rate = (highest_score - second_score) / (highest_score - second_score); // MUTATION m2\n"
assert old in text, "m2 anchor not found"
path.write_text(text.replace(old, new, 1))
PY
}

# --- M3: Mandarin/Japanese threshold strict boundary shifted by one ---------
inject_m3() {
    python3 - "$DETECT_SRC" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
text = path.read_text()
old = "        } else if jpn_pct > 0.05 {\n            (Lang::Jpn, 0.5)\n"
new = "        } else if jpn_pct >= 0.05 { // MUTATION m3\n            (Lang::Jpn, 0.5)\n"
assert old in text, "m3 anchor not found"
path.write_text(text.replace(old, new, 1))
PY
}

inject_m1
run_expect_failure \
    "m1_blacklist_after_ranking" \
    "properties_11_blacklist_never_returns_excluded_languages" \
    "blacklist must be applied before ranking"
restore_sources

inject_m2
run_expect_failure \
    "m2_confidence_denominator" \
    "properties_09_calculate_confidence_helper_is_finite_and_bounded_on_tied_scores" \
    "confidence is always finite and within the public 0.0..=1.0 range"
restore_sources

inject_m3
run_expect_failure \
    "m3_script_threshold_offbyone" \
    "properties_20_mandarin_japanese_threshold_boundaries_are_exact" \
    "mandarin"
restore_sources

clean_workspace
echo "ALL MUTATIONS CAUGHT; production sources restored; workspace clean"
