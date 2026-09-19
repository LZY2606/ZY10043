//! Special-script branches, in particular the Mandarin <-> Japanese hack in
//! `detect`, whose current public contract is percentage based:
//!
//! - kana fraction > 0.20 => Jpn, confidence 1.0
//! - kana fraction > 0.05 => Jpn, confidence 0.5
//! - kana fraction > 0.02 => Cmn, confidence 0.5
//! - otherwise           => Cmn, confidence 1.0
//!
//! Filtering: if Cmn is not allowed, the branch currently always returns Jpn
//! (see `detect_lang_base_on_mandarin_script`). These tests lock the existing
//! public contract rather than any particular natural-language probability.

use super::common::*;

fn han_kana(han: usize, kana: usize) -> String {
    let mut rng = Rng::new(0x6A_7A00);
    let mut chars: Vec<char> = (0..han)
        .map(|_| ScriptKind::Han.random_char(&mut rng))
        .collect();
    for i in 0..kana {
        let kind = if i % 2 == 0 {
            ScriptKind::Hiragana
        } else {
            ScriptKind::Katakana
        };
        chars.push(kind.random_char(&mut rng));
    }
    chars.into_iter().collect()
}

fn expect(text: &str, lang: Lang, confidence: f64, cfg: &FilterCfg) {
    for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
        let detector = cfg.build_detector(method);
        let info = assert_core_invariants(&detector, text).expect("mandarin branch returned None");
        assert_eq!(info.script(), Script::Mandarin);
        assert_eq!(
            info.lang(),
            lang,
            "invariant violated: script threshold boundary off-by-one (mandarin/japanese branch language at boundary)"
        );
        assert_eq!(
            info.confidence(),
            confidence,
            "invariant violated: script threshold boundary off-by-one (mandarin/japanese branch confidence at boundary)"
        );
    }
}

pub(super) fn properties_special_branches_mandarin_pure_han() {
    // 40 han, 0 kana -> Cmn 1.0
    let text = han_kana(40, 0);
    expect(&text, Lang::Cmn, 1.0, &FilterCfg::None);
}

pub(super) fn properties_special_branches_kana_threshold_boundaries() {
    // Exact-boundary *languages*, locked against threshold off-by-one
    // mutations. The 0.20 boundary is the one exercised by the mutation
    // harness (0.20 -> 0.19 must not flip it to Jpn 1.0).
    struct Boundary {
        han: usize,
        kana: usize,
        lang: Lang,
    }
    let boundaries = [
        Boundary {
            han: 49,
            kana: 1,
            lang: Lang::Cmn,
        },
        Boundary {
            han: 39,
            kana: 1,
            lang: Lang::Cmn,
        },
        Boundary {
            han: 18,
            kana: 1,
            lang: Lang::Jpn,
        },
        Boundary {
            han: 51,
            kana: 12,
            lang: Lang::Jpn,
        },
        Boundary {
            han: 39,
            kana: 11,
            lang: Lang::Jpn,
        },
    ];
    for b in boundaries {
        let text = han_kana(b.han, b.kana);
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let info = Detector::new()
                .set_method(method)
                .detect(&text)
                .expect("mandarin branch returned None");
            assert_eq!(
                info.lang(),
                b.lang,
                "invariant violated: script threshold boundary off-by-one (mandarin/japanese branch language at boundary {}/{})",
                b.kana,
                b.han + b.kana
            );
        }
    }
}

pub(super) fn properties_special_branches_kana_threshold_confidence_levels() {
    // Exact-boundary confidence bands, locked separately.
    struct Point {
        han: usize,
        kana: usize,
        lang: Lang,
        confidence: f64,
    }
    let points = [
        Point {
            han: 49,
            kana: 1,
            lang: Lang::Cmn,
            confidence: 1.0,
        },
        Point {
            han: 39,
            kana: 1,
            lang: Lang::Cmn,
            confidence: 0.5,
        },
        Point {
            han: 18,
            kana: 1,
            lang: Lang::Jpn,
            confidence: 0.5,
        },
        Point {
            han: 51,
            kana: 12,
            lang: Lang::Jpn,
            confidence: 0.5,
        },
        Point {
            han: 39,
            kana: 11,
            lang: Lang::Jpn,
            confidence: 1.0,
        },
    ];
    for p in points {
        let text = han_kana(p.han, p.kana);
        expect(&text, p.lang, p.confidence, &FilterCfg::None);
    }
}

pub(super) fn properties_special_branches_allowlist_cmn_or_jpn() {
    let text = han_kana(40, 0);
    // Pure han is indistinguishable Cmn/Jpn; each one-member whitelist
    // must be honored.
    let cmn = FilterCfg::Allow(vec![Lang::Cmn]);
    let jpn = FilterCfg::Allow(vec![Lang::Jpn]);
    for cfg in [&cmn, &jpn] {
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let detector = cfg.build_detector(method);
            let info = assert_core_invariants(&detector, &text).unwrap();
            assert_filter_respected(cfg, Some(&info));
        }
    }
    assert_eq!(
        cmn.build_detector(Method::Combined)
            .detect(&text)
            .unwrap()
            .lang(),
        Lang::Cmn
    );
    assert_eq!(
        jpn.build_detector(Method::Combined)
            .detect(&text)
            .unwrap()
            .lang(),
        Lang::Jpn
    );
}

pub(super) fn properties_special_branches_denylist_current_contract() {
    // Current contract for the Mandarin branch:
    // - denying Jpn keeps the normal percentage path (pure han -> Cmn);
    // - denying Cmn unconditionally falls back to Jpn (even on pure han).
    let text = han_kana(40, 0);
    let deny_jpn = FilterCfg::Deny(vec![Lang::Jpn]);
    expect(&text, Lang::Cmn, 1.0, &deny_jpn);

    let deny_cmn = FilterCfg::Deny(vec![Lang::Cmn]);
    expect(&text, Lang::Jpn, 1.0, &deny_cmn);

    // Empty allowlist hits the same fallback, per current implementation.
    let allow_none = FilterCfg::Allow(vec![]);
    expect(&text, Lang::Jpn, 1.0, &allow_none);
}

pub(super) fn properties_special_branches_random_han_kana_filter_invariants() {
    // Random mixtures: all results must stay inside {Cmn, Jpn} and respect the
    // configured list; the branch is method independent.
    run_fixed_seeds_cfg(&SEEDS, 40, |rng, base| {
        let han = rng.range(1, 40);
        let kana = rng.range(0, 20);
        let text = han_kana(han, kana);
        let cfg = match rng.below(4) {
            0 => FilterCfg::None,
            1 => FilterCfg::Allow(vec![Lang::Cmn, Lang::Jpn]),
            2 => FilterCfg::Allow(vec![*rng.pick(&[Lang::Cmn, Lang::Jpn])]),
            _ => FilterCfg::Deny(vec![*rng.pick(&[Lang::Cmn, Lang::Jpn])]),
        };
        let ctx = CaseCtx {
            filter: cfg.clone(),
            ..base.clone()
        };
        let property: Property = Box::new(move |t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = cfg.build_detector(method);
                if let Some(info) = assert_core_invariants(&detector, t) {
                    if !matches!(info.lang(), Lang::Cmn | Lang::Jpn) {
                        return Err("mandarin branch returned language outside {Cmn,Jpn}".into());
                    }
                    // Current public contract of the Mandarin special branch:
                    // only `Cmn` membership is consulted via `is_allowed`.
                    // A returned `Cmn` must therefore be allowlisted; a
                    // returned `Jpn` is unconditional (denying or not
                    // allowlisting Jpn is ignored today, as is a Cmn result
                    // after the Jpn fallback). The single-language
                    // Hiragana/Katakana -> Jpn branch ignores filters too.
                    if info.script() == Script::Mandarin && info.lang() == Lang::Cmn {
                        assert_filter_respected(&cfg, Some(&info));
                    }
                }
            }
            Ok(())
        });
        let mut stored = text;
        stored.push(' ');
        (ctx, stored, property)
    });
}
