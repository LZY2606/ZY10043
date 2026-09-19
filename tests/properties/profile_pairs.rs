//! Confusable language pairs evaluated against the real trigram profiles.
//!
//! We deliberately do NOT hard-code that natural-language snippets of a
//! language must always win: the model is statistical. The hard requirements
//! are limited to:
//!
//! 1. the same input + configuration is stable across repeated runs;
//! 2. the winner is a legal candidate of the configured filter and belongs to
//!    the shared script family;
//! 3. ranking is self-consistent: forcing the choice inside the pair does not
//!    promote a third language over the unfiltered winner;
//! 4. all three Methods produce finite, in-range confidence.
//!
//! For the record the failure report (and the debug output below) prints the
//! top two languages and the confidence margin.

use super::common::*;

struct Pair {
    name: &'static str,
    a: Lang,
    b: Lang,
    sample_a: &'static str,
    sample_b: &'static str,
}

const PAIRS: &[Pair] = &[
    Pair {
        name: "spa-por",
        a: Lang::Spa,
        b: Lang::Por,
        sample_a: "Y así mismo, aunque no son tan ágiles en el suelo como el vampiro común, \
                   son capaces de recorrer grandes distancias cada noche en busca de alimento.",
        sample_b: "Aliás, no dia oito, antes da estreia do filme no mercado latino, o produtor \
                   português conversou com jornalistas sobre a nova produção.",
    },
    Pair {
        name: "rus-ukr",
        a: Lang::Rus,
        b: Lang::Ukr,
        sample_a: "Происхождение названия села в советское время упоминается в официальных \
                   документах и краеведческих заметках местных жителей.",
        sample_b: "Середньовічний Львів був важливим політичним, економічним і культурним \
                   центром регіону, про що свідчать численні пам'ятки міста.",
    },
    Pair {
        name: "ara-pes",
        a: Lang::Ara,
        b: Lang::Pes,
        sample_a: "عندما يريد العالم أن يتكلّم، فهو يتحدّث بلغة يونيكود، وتسجّل الآن النسخة \
                   العربية من المؤتمر بكثير من الاهتمام.",
        sample_b: "زبان فارسی با گویش‌های گوناگون در منطقه‌ای گسترده گفت‌وگو می‌شود و هنر و \
                   ادبیات کهن ایران زمین وامدار این زبان شیرین است.",
    },
];

fn check_pair(pair: &Pair, sample: &str) {
    for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
        let unfiltered = Detector::new().set_method(method);
        let first = assert_core_invariants(&unfiltered, sample)
            .expect("confusable-pair sample produced no detection");
        let family = first.script().langs().to_vec();
        assert!(
            family.contains(&first.lang()),
            "invariant violated: winner {:?} outside its own script family",
            first.lang()
        );

        // Force the decision inside the pair.
        let pair_cfg = FilterCfg::Allow(vec![pair.a, pair.b]);
        let forced = pair_cfg.build_detector(method);
        let forced_info = assert_core_invariants(&forced, sample)
            .expect("invariant violated: pair-restricted detection returned None");
        assert_filter_respected(&pair_cfg, Some(&forced_info));
        assert!(
            family.contains(&forced_info.lang()),
            "invariant violated: pair winner {:?} not member of the script family",
            forced_info.lang()
        );

        // Self-consistency: the unfiltered winner, when it is one of the pair,
        // must equal the pair-constrained winner.
        if first.lang() == pair.a || first.lang() == pair.b {
            assert_eq!(
                first.lang(),
                forced_info.lang(),
                "invariant violated: pair restriction promoted a different in-pair winner"
            );
        }

        // Order of pair members must not matter.
        let reversed = FilterCfg::Allow(vec![pair.b, pair.a]).build_detector(method);
        let rev_info = assert_core_invariants(&reversed, sample).unwrap();
        assert_eq!(
            (forced_info.lang(), forced_info.confidence().to_bits()),
            (rev_info.lang(), rev_info.confidence().to_bits()),
            "invariant violated: pair ranking depends on whitelist order"
        );

        // Record top two + margin for diagnostics (no hard assertion on which
        // natural language wins).
        let ranked = rank_top_two(method, &FilterCfg::None, sample)
            .expect("ranking should be available for real-script samples");
        assert!(ranked.margin.map(|m| m.is_finite()).unwrap_or(true));
        eprintln!(
            "[profile-pair {}] method={method:?} winner={:?}({}) \
             runner_up={:?}({:?}) margin={:?}",
            pair.name,
            ranked.winner,
            ranked.winner_conf,
            ranked.runner_up,
            ranked.runner_up_conf,
            ranked.margin
        );
    }
}

pub(super) fn properties_profile_pair_spa_por_records_top_two_and_margin() {
    let pair = PAIRS.iter().find(|p| p.name == "spa-por").unwrap();
    check_pair(pair, pair.sample_a);
    check_pair(pair, pair.sample_b);
}

pub(super) fn properties_profile_pair_rus_ukr_records_top_two_and_margin() {
    let pair = PAIRS.iter().find(|p| p.name == "rus-ukr").unwrap();
    check_pair(pair, pair.sample_a);
    check_pair(pair, pair.sample_b);
}

pub(super) fn properties_profile_pair_ara_pes_records_top_two_and_margin() {
    let pair = PAIRS.iter().find(|p| p.name == "ara-pes").unwrap();
    check_pair(pair, pair.sample_a);
    check_pair(pair, pair.sample_b);
}

pub(super) fn properties_mut_target_confidence_normalization_range_trigram() {
    // Mutation target for "wrong confidence normalization denominator":
    // real-profile Trigram scores must stay within the public 0..=1 range.
    let real_samples = [
        "Y así mismo, aunque no son tan ágiles en el suelo como el vampiro común, son capaces de recorrer grandes distancias cada noche.",
        "Происхождение названия села в советское время упоминается в официальных документах и заметках жителей.",
        "عندما يريد العالم أن يتكلم فهو يتحدث بلغة واحدة يفهمها الجميع في كل مكان وزمان.",
        "उन्होंने बताया कि जेब में बहुत सारे रूपए थे और वे उस वक्त वहाँ खड़े थे और बात कर रहे थे।",
        "האקדמיה ללשון העברית עוסקת בחקר הלשון ובהדרכת השימוש בה בכל יום מימות השנה.",
    ];
    for sample in real_samples {
        let detector = Detector::new().set_method(Method::Trigram);
        let info = assert_core_invariants(&detector, sample).expect("real sample must detect");
        assert!(
            info.confidence() <= 1.0 && info.confidence() >= 0.0,
            "invariant violated: confidence normalization produced out-of-range confidence {}",
            info.confidence()
        );
    }
}
