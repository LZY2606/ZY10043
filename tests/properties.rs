//! Fixed-seed property tests for whatlang.
//!
//! These tests never hard-code the statistical model's answer table:
//! generated text asserts *invariants* (determinism, confidence bounds,
//! filter legality, script stability, ordering self-consistency).
//!
//! Run with: `cargo test --quiet properties`
//!
//! This integration target uses a custom (no built-in libtest) harness so
//! that, even under `--quiet`, the names of the named property cases are
//! printed before execution. Each `#[test]` function below is registered in
//! `run_all`, which forwards standard libtest filters (`--exact`, a name
//! substring, `--nocapture`).
//!
//! See `TESTING.md` for seed replay, shrinking and the mutation harness.

#![allow(clippy::needless_raw_string_hashes)]

#[path = "properties/common/mod.rs"]
mod common;

#[path = "properties/mixed.rs"]
mod mixed;

#[path = "properties/profile_pairs.rs"]
mod profile_pairs;

#[path = "properties/scripts_noise.rs"]
mod scripts_noise;

#[path = "properties/special_branches.rs"]
mod special_branches;

#[path = "properties/pure.rs"]
mod pure;

#[path = "properties/filters.rs"]
mod filters;

#[path = "properties/generator.rs"]
mod generator;

use common::PROPERTY_CASE_NAMES;

struct Case {
    name: &'static str,
    run: fn(),
}

fn all_cases() -> Vec<Case> {
    vec![
        Case {
            name: "pure::properties_pure_latin_fragment_invariants",
            run: pure::properties_pure_latin_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_cyrillic_fragment_invariants",
            run: pure::properties_pure_cyrillic_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_arabic_fragment_invariants",
            run: pure::properties_pure_arabic_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_devanagari_fragment_invariants",
            run: pure::properties_pure_devanagari_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_hebrew_fragment_invariants",
            run: pure::properties_pure_hebrew_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_han_fragment_invariants",
            run: pure::properties_pure_han_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_hiragana_fragment_invariants",
            run: pure::properties_pure_hiragana_fragment_invariants,
        },
        Case {
            name: "pure::properties_pure_katakana_fragment_invariants",
            run: pure::properties_pure_katakana_fragment_invariants,
        },
        Case {
            name: "scripts_noise::properties_noise_keeps_latin_script",
            run: scripts_noise::properties_noise_keeps_latin_script,
        },
        Case {
            name: "scripts_noise::properties_noise_keeps_cyrillic_script",
            run: scripts_noise::properties_noise_keeps_cyrillic_script,
        },
        Case {
            name: "scripts_noise::properties_noise_keeps_arabic_script",
            run: scripts_noise::properties_noise_keeps_arabic_script,
        },
        Case {
            name: "scripts_noise::properties_noise_keeps_devanagari_script",
            run: scripts_noise::properties_noise_keeps_devanagari_script,
        },
        Case {
            name: "scripts_noise::properties_noise_keeps_hebrew_script",
            run: scripts_noise::properties_noise_keeps_hebrew_script,
        },
        Case {
            name: "scripts_noise::properties_noise_keeps_han_script",
            run: scripts_noise::properties_noise_keeps_han_script,
        },
        Case {
            name: "mixed::properties_mixed_latin_cyrillic_invariants",
            run: mixed::properties_mixed_latin_cyrillic_invariants,
        },
        Case {
            name: "mixed::properties_mixed_cyrillic_latin_invariants",
            run: mixed::properties_mixed_cyrillic_latin_invariants,
        },
        Case {
            name: "mixed::properties_mixed_arabic_latin_invariants",
            run: mixed::properties_mixed_arabic_latin_invariants,
        },
        Case {
            name: "mixed::properties_mixed_devanagari_latin_invariants",
            run: mixed::properties_mixed_devanagari_latin_invariants,
        },
        Case {
            name: "mixed::properties_mixed_hebrew_latin_invariants",
            run: mixed::properties_mixed_hebrew_latin_invariants,
        },
        Case {
            name: "mixed::properties_mixed_han_kana_invariants",
            run: mixed::properties_mixed_han_kana_invariants,
        },
        Case {
            name: "filters::properties_filters_random_allowlist_membership",
            run: filters::properties_filters_random_allowlist_membership,
        },
        Case {
            name: "filters::properties_filters_random_denylist_exclusion",
            run: filters::properties_filters_random_denylist_exclusion,
        },
        Case {
            name: "filters::properties_filters_matrix_sizes_and_member_order",
            run: filters::properties_filters_matrix_sizes_and_member_order,
        },
        Case {
            name: "filters::properties_filters_empty_allowlist_contract",
            run: filters::properties_filters_empty_allowlist_contract,
        },
        Case {
            name: "filters::properties_filters_single_candidate_contract",
            run: filters::properties_filters_single_candidate_contract,
        },
        Case {
            name: "filters::properties_filters_denylist_all_family_members_contract",
            run: filters::properties_filters_denylist_all_family_members_contract,
        },
        Case {
            name: "filters::properties_mut_target_denylist_applied_before_ranking",
            run: filters::properties_mut_target_denylist_applied_before_ranking,
        },
        Case {
            name: "special_branches::properties_special_branches_mandarin_pure_han",
            run: special_branches::properties_special_branches_mandarin_pure_han,
        },
        Case {
            name: "special_branches::properties_special_branches_kana_threshold_boundaries",
            run: special_branches::properties_special_branches_kana_threshold_boundaries,
        },
        Case {
            name: "special_branches::properties_special_branches_kana_threshold_confidence_levels",
            run: special_branches::properties_special_branches_kana_threshold_confidence_levels,
        },
        Case {
            name: "special_branches::properties_special_branches_allowlist_cmn_or_jpn",
            run: special_branches::properties_special_branches_allowlist_cmn_or_jpn,
        },
        Case {
            name: "special_branches::properties_special_branches_denylist_current_contract",
            run: special_branches::properties_special_branches_denylist_current_contract,
        },
        Case {
            name: "special_branches::properties_special_branches_random_han_kana_filter_invariants",
            run: special_branches::properties_special_branches_random_han_kana_filter_invariants,
        },
        Case {
            name: "profile_pairs::properties_profile_pair_spa_por_records_top_two_and_margin",
            run: profile_pairs::properties_profile_pair_spa_por_records_top_two_and_margin,
        },
        Case {
            name: "profile_pairs::properties_profile_pair_rus_ukr_records_top_two_and_margin",
            run: profile_pairs::properties_profile_pair_rus_ukr_records_top_two_and_margin,
        },
        Case {
            name: "profile_pairs::properties_profile_pair_ara_pes_records_top_two_and_margin",
            run: profile_pairs::properties_profile_pair_ara_pes_records_top_two_and_margin,
        },
        Case {
            name: "profile_pairs::properties_mut_target_confidence_normalization_range_trigram",
            run: profile_pairs::properties_mut_target_confidence_normalization_range_trigram,
        },
        Case {
            name: "generator::properties_generator_sequence_is_replayable_in_order",
            run: generator::properties_generator_sequence_is_replayable_in_order,
        },
        Case {
            name: "generator::properties_generator_seed_override_env",
            run: generator::properties_generator_seed_override_env,
        },
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut exact = false;
    let mut list_only = false;
    let mut filters: Vec<String> = Vec::new();
    for arg in &args {
        match arg.as_str() {
            "--exact" => exact = true,
            "--list" | "--list-tests" => list_only = true,
            "--nocapture" | "--test-threads" | "--color" | "--format" => {}
            a if a.starts_with("--test-threads=")
                || a.starts_with("--color=")
                || a.starts_with("--format=") => {}
            a if !a.starts_with('-') => filters.push(a.to_string()),
            _ => {}
        }
    }

    let cases = all_cases();

    // Keep the catalog exported from the shared helpers in sync with the
    // registered cases (same set, same order).
    assert_eq!(
        PROPERTY_CASE_NAMES.len(),
        cases.len(),
        "PROPERTY_CASE_NAMES out of sync with registered cases"
    );
    for (registered, exported) in cases.iter().zip(PROPERTY_CASE_NAMES.iter()) {
        assert_eq!(registered.name, *exported, "case catalog order mismatch");
    }

    println!("running {} named property tests", cases.len());
    for case in &cases {
        println!("test {} ...", case.name);
    }

    if list_only {
        return;
    }

    let selected: Vec<&Case> = cases
        .iter()
        .filter(|case| {
            filters.is_empty()
                || filters.iter().any(|f| {
                    if exact {
                        case.name == f
                    } else {
                        case.name.contains(f)
                    }
                })
        })
        .collect();

    let mut failed = 0usize;
    for case in &selected {
        print!("test {} ... ", case.name);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let started = std::time::Instant::now();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(case.run));
        let elapsed = started.elapsed();
        match outcome {
            Ok(()) => println!("ok ({:.2}s)", elapsed.as_secs_f64()),
            Err(_) => {
                println!("FAILED ({:.2}s)", elapsed.as_secs_f64());
                failed += 1;
            }
        }
    }

    println!();
    println!(
        "test result: {}. {} passed; {} failed; 0 ignored; 0 measured; {} filtered out",
        if failed == 0 { "ok" } else { "FAILED" },
        selected.len() - failed,
        failed,
        cases.len() - selected.len()
    );

    if failed != 0 {
        std::process::exit(1);
    }
}
