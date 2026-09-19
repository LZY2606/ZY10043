//! Filter matrix invariants across all supported Methods:
//! - whitelist results come only from the allowed set;
//! - denylist never returns an excluded member;
//! - same members in different order agree;
//! - single candidate obeys the public single-candidate contract;
//! - empty candidate sets obey the current public contract;
//! - allow and deny are mutually exclusive configurations, each enforced.

use super::common::*;

const CASES_PER_SEED: usize = 16;

/// Multi-script language families exercised by the matrix (each maps to a
/// `MultiLangScript` profile group).
const FAMILIES: &[(ScriptKind, &[Lang])] = &[
    (
        ScriptKind::Latin,
        &[
            Lang::Eng,
            Lang::Deu,
            Lang::Fra,
            Lang::Spa,
            Lang::Por,
            Lang::Ita,
            Lang::Nld,
            Lang::Pol,
            Lang::Epo,
            Lang::Tgl,
        ],
    ),
    (
        ScriptKind::Cyrillic,
        &[
            Lang::Rus,
            Lang::Ukr,
            Lang::Srp,
            Lang::Bel,
            Lang::Bul,
            Lang::Mkd,
        ],
    ),
    (ScriptKind::Arabic, &[Lang::Ara, Lang::Urd, Lang::Pes]),
    (ScriptKind::Devanagari, &[Lang::Hin, Lang::Mar, Lang::Nep]),
    (ScriptKind::Hebrew, &[Lang::Heb, Lang::Yid]),
];

pub(super) fn properties_filters_random_allowlist_membership() {
    run_fixed_seeds_cfg(&SEEDS, CASES_PER_SEED, |rng, base| {
        let (kind, pool) = rng.pick(FAMILIES);
        let text = gen_pure_fragment(rng, *kind);
        let members = random_subset(rng, pool, 5);
        let cfg = FilterCfg::Allow(members);
        let ctx = CaseCtx {
            filter: cfg.clone(),
            ..base.clone()
        };
        let property: Property = Box::new(move |t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = cfg.build_detector(method);
                let info = assert_core_invariants(&detector, t);
                assert_filter_respected(&cfg, info.as_ref());
            }
            Ok(())
        });
        (ctx, text, property)
    });
}

pub(super) fn properties_filters_random_denylist_exclusion() {
    run_fixed_seeds_cfg(&SEEDS, CASES_PER_SEED, |rng, base| {
        let (kind, pool) = rng.pick(FAMILIES);
        let text = gen_pure_fragment(rng, *kind);
        let members = random_subset(rng, pool, 4);
        let cfg = FilterCfg::Deny(members);
        let ctx = CaseCtx {
            filter: cfg.clone(),
            ..base.clone()
        };
        let property: Property = Box::new(move |t| {
            for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                let detector = cfg.build_detector(method);
                let info = assert_core_invariants(&detector, t);
                assert_filter_respected(&cfg, info.as_ref());
            }
            Ok(())
        });
        (ctx, text, property)
    });
}

pub(super) fn properties_filters_matrix_sizes_and_member_order() {
    // Deterministic matrix: sizes 0, 1, 2, family/2 and family size, allow and
    // deny, original and reversed member order.
    for (kind, pool) in FAMILIES {
        let sizes = [0usize, 1, 2, pool.len() / 2, pool.len()];
        for size in sizes {
            for cfg in sized_filters(pool, size) {
                let mut rng = Rng::new(0xF1_17E0 + size as u64 * 7 + pool.len() as u64);
                let text = gen_pure_fragment(&mut rng, *kind);
                for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
                    let detector = cfg.build_detector(method);
                    let info = assert_core_invariants(&detector, &text);
                    assert_filter_respected(&cfg, info.as_ref());
                }
            }
        }

        // Same members in a different order must agree for both list kinds.
        let members: Vec<Lang> = pool.to_vec();
        let mut reversed = members.clone();
        reversed.reverse();
        let mut rng = Rng::new(0xA11CE5);
        let text = gen_pure_fragment(&mut rng, *kind);
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let a = FilterCfg::Allow(members.clone()).build_detector(method);
            let b = FilterCfg::Allow(reversed.clone()).build_detector(method);
            let ia = a.detect(&text);
            let ib = b.detect(&text);
            assert_eq!(
                ia.map(|i| (i.lang(), i.script(), i.confidence().to_bits())),
                ib.map(|i| (i.lang(), i.script(), i.confidence().to_bits())),
                "invariant violated: allowlist results depend on member order ({kind:?}, {method:?})"
            );

            let da = FilterCfg::Deny(members.clone()).build_detector(method);
            let db = FilterCfg::Deny(reversed.clone()).build_detector(method);
            let ida = da.detect(&text);
            let idb = db.detect(&text);
            assert_eq!(
                ida.map(|i| (i.lang(), i.script(), i.confidence().to_bits())),
                idb.map(|i| (i.lang(), i.script(), i.confidence().to_bits())),
                "invariant violated: denylist results depend on member order ({kind:?}, {method:?})"
            );
        }
    }
}

pub(super) fn properties_filters_empty_allowlist_contract() {
    // Current public contract: an empty whitelist leaves no candidates for
    // the multi-language script families, so detection returns None.
    for (kind, _pool) in FAMILIES {
        let mut rng = Rng::new(0xE07_1157);
        let text = gen_pure_fragment(&mut rng, *kind);
        let cfg = FilterCfg::Allow(vec![]);
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let detector = cfg.build_detector(method);
            let info = assert_core_invariants(&detector, &text);
            assert_eq!(
                info, None,
                "invariant violated: empty allowlist produced a language for {kind:?} under {method:?}"
            );
        }
    }
}

pub(super) fn properties_filters_single_candidate_contract() {
    // A whitelist of one language locks the result to that language with
    // confidence 1.0 for every multi-language family and every Method.
    for (kind, pool) in FAMILIES {
        let lang = pool[0];
        let mut rng = Rng::new(0x0051_461E);
        let text = gen_pure_fragment(&mut rng, *kind);
        let cfg = FilterCfg::Allow(vec![lang]);
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let detector = cfg.build_detector(method);
            assert_single_candidate_contract(&detector, lang, &text);
        }
    }
}

pub(super) fn properties_filters_denylist_all_family_members_contract() {
    // Denying every language of a family (the complete public
    // `Script::langs()` set) leaves no candidate: None.
    for (kind, _pool) in FAMILIES {
        let mut rng = Rng::new(0xDE_1A11);
        let text = gen_pure_fragment(&mut rng, *kind);
        let cfg = FilterCfg::Deny(kind.main_script().langs().to_vec());
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let detector = cfg.build_detector(method);
            let info = assert_core_invariants(&detector, &text);
            assert_eq!(
                info, None,
                "invariant violated: denying all {kind:?} languages still returned one ({method:?})"
            );
        }
    }
}

pub(super) fn properties_mut_target_denylist_applied_before_ranking() {
    // Mutation target for "denylist applied only after ranking".
    // On every family the Trigram winner must be replaceable by denying it:
    // filtering happens before ranking, so a denied language can never win.
    let samples = [
        (
            ScriptKind::Latin,
            "The quick brown fox jumps over the lazy dog while the morning wind moves through the trees",
        ),
        (
            ScriptKind::Cyrillic,
            "Происхождение названия села в советское время упоминается в официальных документах",
        ),
        (
            ScriptKind::Arabic,
            "عندما يريد العالم أن يتكلم فهو يتحدث بلغة واحدة يفهمها الجميع في كل مكان",
        ),
        (
            ScriptKind::Devanagari,
            "उन्होंने बताया कि जेब में बहुत सारे रूपए थे और वे उस वक्त वहाँ खड़े थे",
        ),
        (
            ScriptKind::Hebrew,
            "האקדמיה ללשון העברית עוסקת בחקר הלשון ובהדרכת השימוש בה בכל יום",
        ),
    ];
    for (kind, sample) in samples {
        let winner = Detector::new()
            .set_method(Method::Trigram)
            .detect(sample)
            .expect("sample must detect")
            .lang();
        let cfg = FilterCfg::Deny(vec![winner]);
        let detector = cfg.build_detector(Method::Trigram);
        let info = detector.detect(sample);
        if let Some(info) = info {
            assert_ne!(
                info.lang(),
                winner,
                "invariant violated: denylist applied after ranking, excluded {winner:?} still won ({kind:?})",
            );
        }
    }
}
