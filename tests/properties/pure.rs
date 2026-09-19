//! Pure single-script fragments: determinism, finite in-range confidence and
//! script detection. Language itself is never asserted for the ambiguous
//! generated samples.

use super::common::*;

const CASES_PER_SEED: usize = 24;

fn pure_property(method: Method, kind: ScriptKind, text: &str) -> Result<(), String> {
    let detector = Detector::new().set_method(method);
    let info = assert_core_invariants(&detector, text)
        .ok_or_else(|| "pure script fragment produced no detection".to_string())?;

    let detected_script = whatlang::detect_script(text)
        .ok_or_else(|| "pure script fragment produced no script".to_string())?;
    if detected_script != kind.main_script() {
        return Err(format!(
            "pure {kind} fragment detected as {detected_script:?}"
        ));
    }
    if info.script() != kind.main_script() {
        return Err(format!(
            "info.script() {:?} disagrees with pure fragment script {:?}",
            info.script(),
            kind.main_script()
        ));
    }
    Ok(())
}

fn run_pure(kind: ScriptKind) {
    run_fixed_seeds(&SEEDS, CASES_PER_SEED, |rng, ctx| {
        let text = gen_pure_fragment(rng, kind);
        let method = ctx.method;
        let property: Property = Box::new(move |t| pure_property(method, kind, t));
        (text, property)
    });
}

pub(super) fn properties_pure_latin_fragment_invariants() {
    run_pure(ScriptKind::Latin);
}

pub(super) fn properties_pure_cyrillic_fragment_invariants() {
    run_pure(ScriptKind::Cyrillic);
}

pub(super) fn properties_pure_arabic_fragment_invariants() {
    run_pure(ScriptKind::Arabic);
}

pub(super) fn properties_pure_devanagari_fragment_invariants() {
    run_pure(ScriptKind::Devanagari);
}

pub(super) fn properties_pure_hebrew_fragment_invariants() {
    run_pure(ScriptKind::Hebrew);
}

pub(super) fn properties_pure_han_fragment_invariants() {
    run_pure(ScriptKind::Han);
}

pub(super) fn properties_pure_hiragana_fragment_invariants() {
    // Hiragana maps to the single-language Jpn branch: assert the current
    // public contract (Jpn, confidence 1.0) instead of statistical guesses.
    run_fixed_seeds(&SEEDS, CASES_PER_SEED, |rng, _ctx| {
        let text = gen_pure_fragment(rng, ScriptKind::Hiragana);
        let property: Property = Box::new(|t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = Detector::new().set_method(method);
                let info = assert_core_invariants(&detector, t)
                    .ok_or("hiragana fragment produced no detection")?;
                if info.lang() != Lang::Jpn || info.script() != Script::Hiragana {
                    return Err("hiragana must map to Lang::Jpn / Script::Hiragana".into());
                }
            }
            Ok(())
        });
        (text, property)
    });
}

pub(super) fn properties_pure_katakana_fragment_invariants() {
    run_fixed_seeds(&SEEDS, CASES_PER_SEED, |rng, _ctx| {
        let text = gen_pure_fragment(rng, ScriptKind::Katakana);
        let property: Property = Box::new(|t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = Detector::new().set_method(method);
                let info = assert_core_invariants(&detector, t)
                    .ok_or("katakana fragment produced no detection")?;
                if info.lang() != Lang::Jpn || info.script() != Script::Katakana {
                    return Err("katakana must map to Lang::Jpn / Script::Katakana".into());
                }
            }
            Ok(())
        });
        (text, property)
    });
}
