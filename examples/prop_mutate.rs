//! Automated mutation entry for the fixed-seed property suite.
//!
//! Run from the repository root:
//!
//! ```sh
//! cargo run --example prop_mutate
//! ```
//!
//! The harness applies three independent, pre-defined source mutations that
//! each mimic a realistic defect class:
//!
//! 1. `denylist_applied_after_ranking`: the trigram path stops filtering
//!    candidates *before* ranking and instead hides the winner only after
//!    scores are computed, so an excluded language can still win.
//! 2. `wrong_confidence_normalization_denominator`: when the margin rate clears
//!    the hyperbola threshold, normalized confidence must clamp to 1.0; the
//!    mutation returns the raw unbounded rate instead, producing confidence > 1.
//! 3. `script_threshold_boundary_off_by_one`: the Mandarin/Japanese 20%
//!    threshold is relaxed to 19%, shifting the exact-boundary contract.
//!
//! For every mutation the harness:
//! - runs a targeted property-test binary (must fail) and checks the failure
//!   message names the broken invariant;
//! - restores the source from backup immediately afterwards;
//! - re-runs the full properties suite on restored code (must pass);
//! - finally verifies the working tree is clean (`git status --porcelain`).
//!
//! Exit code is zero only if every mutation was caught AND the tree is clean
//! at the end.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

struct Mutation {
    name: &'static str,
    file: &'static str,
    original: &'static str,
    mutated: &'static str,
    test_filter: &'static str,
    expect_in_stderr: &'static [&'static str],
}

const MUTATIONS: &[Mutation] = &[
    Mutation {
        name: "denylist_applied_after_ranking",
        file: "src/trigrams/detection.rs",
        // Candidate filtering must happen while iterating profiles, i.e.
        // BEFORE ranking. The mutation drops the pre-filter so denied
        // languages participate in ranking.
        original: "    for &(lang, lang_trigrams) in lang_profile_list {\n        if !filter_list.is_allowed(lang) {\n            continue;\n        }\n        let dist = calculate_distance(lang_trigrams, &trigram_positions);",
        mutated: "    for &(lang, lang_trigrams) in lang_profile_list {\n        let dist = calculate_distance(lang_trigrams, &trigram_positions);",
        test_filter: "properties_mut_target_denylist_applied_before_ranking",
        expect_in_stderr: &["invariant violated", "denylist applied after ranking"],
    },
    Mutation {
        name: "wrong_confidence_normalization_denominator",
        file: "src/core/confidence.rs",
        // When the margin rate clears the hyperbola threshold, normalized
        // confidence must clamp to 1.0. Returning the raw, unbounded rate is
        // a normalization defect that yields confidence > 1.
        original: "    if rate > confident_rate {\n        1.0\n    } else {",
        mutated: "    if rate > confident_rate {\n        rate\n    } else {",
        test_filter: "properties_mut_target_confidence_normalization_range_trigram",
        expect_in_stderr: &[
            "invariant violated",
            "confidence",
            "is outside the public 0..=1 range",
        ],
    },
    Mutation {
        name: "script_threshold_boundary_off_by_one",
        file: "src/core/detect.rs",
        // Exact-boundary contract: kana fraction must be strictly greater
        // than 0.20; lowering to 0.19 is a classic off-by-one at the boundary.
        original: "        if jpn_pct > 0.2 {",
        mutated: "        if jpn_pct > 0.19 {",
        test_filter: "properties_special_branches_kana_threshold_confidence_levels",
        expect_in_stderr: &[
            "invariant violated",
            "mandarin/japanese branch confidence at boundary",
        ],
    },
];

struct FileGuard<'a> {
    path: &'a str,
    original: String,
    restored: bool,
}

impl<'a> FileGuard<'a> {
    fn capture(path: &'a str) -> Self {
        let original = fs::read_to_string(path).expect("read source file for mutation");
        FileGuard {
            path,
            original,
            restored: false,
        }
    }

    fn restore(&mut self) {
        if !self.restored {
            fs::write(self.path, &self.original).expect("restore mutated source file");
            self.restored = true;
        }
    }
}

impl Drop for FileGuard<'_> {
    fn drop(&mut self) {
        self.restore();
    }
}

fn run(cmd: &mut Command) -> (bool, String) {
    let output = cmd
        .stdin(Stdio::null())
        .output()
        .expect("spawn cargo/git from mutation harness");
    let mut merged = String::new();
    merged.push_str(&String::from_utf8_lossy(&output.stdout));
    merged.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), merged)
}

fn assert_clean_tree() {
    // Only the files touched by the defined mutations must be clean; the
    // harness must never leave them modified.
    let paths: Vec<&str> = MUTATIONS.iter().map(|mutation| mutation.file).collect();
    let mut args = vec!["status", "--porcelain", "--"];
    args.extend(paths);
    let (ok, out) = run(Command::new("git").args(&args));
    assert!(ok, "git status failed");
    let dirty = out.trim().to_string();
    assert!(
        dirty.is_empty(),
        "mutation harness left the working tree dirty:\n{dirty}"
    );
}

fn main() {
    // Restore guard for the whole run in case of an early panic.
    let _workdir_check = Path::new("Cargo.toml").exists();
    assert!(
        _workdir_check,
        "run the mutation harness from the repository root"
    );

    let mut failures = Vec::new();

    for mutation in MUTATIONS {
        println!("== mutation: {} ==", mutation.name);

        let mut guard = FileGuard::capture(mutation.file);

        // 1. Sanity check baseline: targeted test must pass unmutated.
        let (baseline_ok, baseline_out) = run(Command::new("cargo").args([
            "test",
            "--quiet",
            "--test",
            "properties",
            "--",
            mutation.test_filter,
        ]));
        if !baseline_ok {
            failures.push(format!(
                "{}: baseline test fails before mutation (unexpected)\n{baseline_out}",
                mutation.name
            ));
            guard.restore();
            continue;
        }

        // 2. Apply the mutation exactly once.
        let source = fs::read_to_string(mutation.file).unwrap();
        let occurrences = source.matches(mutation.original).count();
        assert_eq!(
            occurrences, 1,
            "mutation anchor for {} must occur exactly once in {} (found {occurrences})",
            mutation.name, mutation.file
        );
        let mutated_source = source.replacen(mutation.original, mutation.mutated, 1);
        fs::write(mutation.file, mutated_source).unwrap();

        // 3. The targeted property test must fail and name the invariant.
        let (caught, out) = run(Command::new("cargo").args([
            "test",
            "--quiet",
            "--test",
            "properties",
            "--",
            mutation.test_filter,
        ]));
        let caught = !caught; // test process is expected to exit non-zero
        let mentions_invariant = mutation
            .expect_in_stderr
            .iter()
            .all(|needle| out.contains(needle));
        if caught && mentions_invariant {
            println!(
                "   caught by `{}` (invariant reported)",
                mutation.test_filter
            );
        } else {
            failures.push(format!(
                "{}: NOT caught (test_failed={caught}, invariant_named={mentions_invariant})\n{out}",
                mutation.name
            ));
        }

        // 4. Restore source immediately (guard restores again on drop).
        guard.restore();

        // 5. Targeted test must pass again after restoration.
        let (restored_ok, restored_out) = run(Command::new("cargo").args([
            "test",
            "--quiet",
            "--test",
            "properties",
            "--",
            mutation.test_filter,
        ]));
        if !restored_ok {
            failures.push(format!(
                "{}: test still fails after restore\n{restored_out}",
                mutation.name
            ));
        }
    }

    // Full properties suite on fully restored code, then clean-tree check.
    let (all_ok, all_out) = run(Command::new("cargo").args(["test", "--quiet", "properties"]));
    if !all_ok {
        failures.push(format!(
            "full properties suite fails after restore:\n{all_out}"
        ));
    }
    assert_clean_tree();

    if failures.is_empty() {
        println!("\nAll 3 mutations were caught, source restored and tree is clean.");
    } else {
        eprintln!("\nMUTATION HARNESS FAILURES:");
        for failure in &failures {
            eprintln!("- {failure}\n");
        }
        std::process::exit(1);
    }
}
