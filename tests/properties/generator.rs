//! Fixed-seed replay: the generation pipeline must produce the same sequence
//! of cases on every run (no HashMap order, no system entropy).

use super::common::*;

fn build_ledger() -> String {
    let mut ledger = String::new();
    for (si, kind) in ScriptKind::all().iter().enumerate() {
        for seed in SEEDS {
            for iteration in 0..6 {
                let mut rng = Rng::new(
                    seed.wrapping_add(iteration as u64 * 0x9E37)
                        .wrapping_add(si as u64 * 0x1000),
                );
                let pure = gen_pure_fragment(&mut rng, *kind);
                let spec = MixedSpec {
                    primary: *kind,
                    secondary: ScriptKind::Latin,
                    secondary_count: 1 + iteration,
                };
                let mixed = gen_mixed_fragment(&mut rng, &spec);
                let noisy = decorate_with_noise(&mut rng, &pure).text;
                ledger.push_str(&format!(
                    "{}|{}|{}|{}|{}|{}\n",
                    kind.name(),
                    seed,
                    iteration,
                    codepoints(&pure),
                    codepoints(&mixed),
                    codepoints(&noisy)
                ));
            }
        }
    }
    ledger
}

pub(super) fn properties_generator_sequence_is_replayable_in_order() {
    let first = build_ledger();
    let second = build_ledger();
    assert_eq!(
        first, second,
        "invariant violated: fixed-seed generation order is not stable across replays"
    );
}

pub(super) fn properties_generator_seed_override_env() {
    // Documents the replay knob: WHATLANG_PROP_SEED replaces the seed list
    // with a single fixed seed (handy for local minimization). The generated
    // case must still satisfy core invariants.
    let seed = std::env::var("WHATLANG_PROP_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0x5EED_0001);
    let mut rng = Rng::new(seed);
    let kind = *rng.pick(ScriptKind::all());
    let text = gen_pure_fragment(&mut rng, kind);
    let detector = Detector::new();
    let _ = assert_core_invariants(&detector, &text);
}
