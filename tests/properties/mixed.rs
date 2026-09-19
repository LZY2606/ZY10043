//! Mixed-script fragments. These can legitimately resolve to either script,
//! so only core invariants (determinism + finite confidence in public range)
//! across every supported detection Method are asserted.

use super::common::*;

const CASES_PER_SEED: usize = 24;

fn run_mixed(primary: ScriptKind, secondary: ScriptKind) {
    run_fixed_seeds(&SEEDS, CASES_PER_SEED, |rng, _ctx| {
        let injected = rng.range(1, 12);
        let spec = MixedSpec {
            primary,
            secondary,
            secondary_count: injected,
        };
        let text = gen_mixed_fragment(rng, &spec);
        let property: Property = Box::new(move |t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = Detector::new().set_method(method);
                let _ = assert_core_invariants(&detector, t);
            }
            Ok(())
        });
        (text, property)
    });
}

pub(super) fn properties_mixed_latin_cyrillic_invariants() {
    run_mixed(ScriptKind::Latin, ScriptKind::Cyrillic);
}

pub(super) fn properties_mixed_cyrillic_latin_invariants() {
    run_mixed(ScriptKind::Cyrillic, ScriptKind::Latin);
}

pub(super) fn properties_mixed_arabic_latin_invariants() {
    run_mixed(ScriptKind::Arabic, ScriptKind::Latin);
}

pub(super) fn properties_mixed_devanagari_latin_invariants() {
    run_mixed(ScriptKind::Devanagari, ScriptKind::Latin);
}

pub(super) fn properties_mixed_hebrew_latin_invariants() {
    run_mixed(ScriptKind::Hebrew, ScriptKind::Latin);
}

pub(super) fn properties_mixed_han_kana_invariants() {
    // Han with Hiragana/Katakana injections: exercise the Mandarin/Japanese
    // branch under all Methods without locking the percentage thresholds here
    // (thresholds are locked separately in special_branches.rs).
    run_fixed_seeds(&SEEDS, CASES_PER_SEED, |rng, _ctx| {
        let primary = ScriptKind::Han;
        let secondary = if rng.chance(1, 2) {
            ScriptKind::Hiragana
        } else {
            ScriptKind::Katakana
        };
        let injected = rng.range(1, 20);
        let spec = MixedSpec {
            primary,
            secondary,
            secondary_count: injected,
        };
        let text = gen_mixed_fragment(rng, &spec);
        let property: Property = Box::new(move |t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = Detector::new().set_method(method);
                if let Some(info) = assert_core_invariants(&detector, t) {
                    if !matches!(
                        info.script(),
                        Script::Mandarin | Script::Hiragana | Script::Katakana
                    ) {
                        return Err(format!(
                            "han/kana mixture resolved to unexpected script {:?}",
                            info.script()
                        ));
                    }
                    if !matches!(info.lang(), Lang::Cmn | Lang::Jpn) {
                        return Err(format!(
                            "han/kana mixture resolved to unexpected language {:?}",
                            info.lang()
                        ));
                    }
                }
            }
            Ok(())
        });
        (text, property)
    });
}
