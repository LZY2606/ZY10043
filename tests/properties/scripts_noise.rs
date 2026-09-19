//! Pure single-script fragments decorated with punctuation, digits, various
//! line breaks and leading/trailing whitespace must keep the same script
//! verdict. Every noise character is an ASCII stop-character, so it never
//! contributes to any script counter.

use super::common::*;

const CASES_PER_SEED: usize = 20;

fn run_noise_stability(kind: ScriptKind) {
    run_fixed_seeds(&SEEDS, CASES_PER_SEED, |rng, _ctx| {
        let base = gen_pure_fragment(rng, kind);
        let noisy = decorate_with_noise(rng, &base).text;
        let expected = kind.main_script();
        let property: Property = Box::new(move |t| {
            let script = whatlang::detect_script(t)
                .ok_or_else(|| "noise-decorated pure fragment lost its script".to_string())?;
            if script != expected {
                return Err(format!(
                    "script changed from {expected:?} to {script:?} after noise decoration"
                ));
            }
            Ok(())
        });
        (noisy, property)
    });
}

pub(super) fn properties_noise_keeps_latin_script() {
    run_noise_stability(ScriptKind::Latin);
}

pub(super) fn properties_noise_keeps_cyrillic_script() {
    run_noise_stability(ScriptKind::Cyrillic);
}

pub(super) fn properties_noise_keeps_arabic_script() {
    run_noise_stability(ScriptKind::Arabic);
}

pub(super) fn properties_noise_keeps_devanagari_script() {
    run_noise_stability(ScriptKind::Devanagari);
}

pub(super) fn properties_noise_keeps_hebrew_script() {
    run_noise_stability(ScriptKind::Hebrew);
}

pub(super) fn properties_noise_keeps_han_script() {
    run_noise_stability(ScriptKind::Han);
}
