//! Shared, dependency-free helpers for the fixed-seed property suite.
//!
//! Nothing here reaches into private production API: every assertion goes
//! through the publicly exported surface (`detect`, `Detector`, `Info`,
//! `Method`, `Lang`, `Script`).

pub use whatlang::{Detector, Info, Lang, Method, Script};

// ---------------------------------------------------------------------------
// Deterministic pseudo random generator (SplitMix64 based)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound.max(1) as u64) as usize
    }

    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi.saturating_sub(lo) + 1)
    }

    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        (self.next_u64() % denominator as u64) < numerator as u64
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Fixed seeds shared by all property tests so two runs generate the exact
/// same cases.
pub const SEEDS: [u64; 8] = [
    0x5EED_0001,
    0x5EED_0002,
    0x5EED_0003,
    0x5EED_0004,
    0x5EED_0005,
    0x5EED_0006,
    0x5EED_0007,
    0x5EED_0008,
];

// ---------------------------------------------------------------------------
// Script character sets (restricted code point ranges)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptKind {
    Latin,
    Cyrillic,
    Arabic,
    Devanagari,
    Hebrew,
    Han,
    Hiragana,
    Katakana,
}

impl ScriptKind {
    pub fn all() -> &'static [ScriptKind] {
        &[
            ScriptKind::Latin,
            ScriptKind::Cyrillic,
            ScriptKind::Arabic,
            ScriptKind::Devanagari,
            ScriptKind::Hebrew,
            ScriptKind::Han,
            ScriptKind::Hiragana,
            ScriptKind::Katakana,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            ScriptKind::Latin => "Latin",
            ScriptKind::Cyrillic => "Cyrillic",
            ScriptKind::Arabic => "Arabic",
            ScriptKind::Devanagari => "Devanagari",
            ScriptKind::Hebrew => "Hebrew",
            ScriptKind::Han => "Han",
            ScriptKind::Hiragana => "Hiragana",
            ScriptKind::Katakana => "Katakana",
        }
    }

    /// Script reported by `whatlang::detect_script` for pure fragments.
    pub fn main_script(self) -> Script {
        match self {
            ScriptKind::Latin => Script::Latin,
            ScriptKind::Cyrillic => Script::Cyrillic,
            ScriptKind::Arabic => Script::Arabic,
            ScriptKind::Devanagari => Script::Devanagari,
            ScriptKind::Hebrew => Script::Hebrew,
            ScriptKind::Han => Script::Mandarin,
            ScriptKind::Hiragana => Script::Hiragana,
            ScriptKind::Katakana => Script::Katakana,
        }
    }

    fn ranges(self) -> &'static [(u32, u32)] {
        match self {
            ScriptKind::Latin => &[
                (0x0061, 0x007A),
                (0x0041, 0x005A),
                (0x00C0, 0x00FF),
                (0x0100, 0x017F),
                (0x0180, 0x024F),
            ],
            ScriptKind::Cyrillic => &[(0x0400, 0x0484), (0x0487, 0x04FF)],
            ScriptKind::Arabic => &[
                (0x0620, 0x063F),
                (0x0641, 0x064A),
                (0x066E, 0x066F),
                (0x0671, 0x06D3),
                (0xFB50, 0xFBC1),
            ],
            ScriptKind::Devanagari => &[(0x0904, 0x0939), (0x093E, 0x094F), (0x0958, 0x096F)],
            ScriptKind::Hebrew => &[(0x05D0, 0x05EA), (0x05B0, 0x05C7), (0x05F0, 0x05F4)],
            ScriptKind::Han => &[(0x4E00, 0x9FCC)],
            ScriptKind::Hiragana => &[(0x3041, 0x3096)],
            ScriptKind::Katakana => &[(0x30A1, 0x30FA), (0x30FC, 0x30FF)],
        }
    }

    pub fn char_count(self) -> usize {
        self.ranges()
            .iter()
            .map(|(lo, hi)| (hi - lo + 1) as usize)
            .sum()
    }

    pub fn char_at(self, index: usize) -> char {
        let mut idx = index;
        for &(lo, hi) in self.ranges() {
            let len = (hi - lo + 1) as usize;
            if idx < len {
                return char::from_u32(lo + idx as u32).expect("valid code point in range");
            }
            idx -= len;
        }
        panic!("char_at index out of range for {}", self.name());
    }

    pub fn random_char(self, rng: &mut Rng) -> char {
        self.char_at(rng.below(self.char_count()))
    }
}

impl std::fmt::Display for ScriptKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

// ---------------------------------------------------------------------------
// Fragment generation (bounded length, restricted character sets)
// ---------------------------------------------------------------------------

pub const MIN_FRAGMENT_LEN: usize = 1;
pub const MAX_FRAGMENT_LEN: usize = 40;

/// Generate a bounded fragment consisting only of characters of `script`.
/// Words are separated with a plain ASCII space (a stop-char for the
/// detector), which never contributes to any script counter.
pub fn gen_pure_fragment(rng: &mut Rng, script: ScriptKind) -> String {
    let len = rng.range(MIN_FRAGMENT_LEN, MAX_FRAGMENT_LEN);
    let mut out = String::new();
    for i in 0..len {
        if i > 0 && rng.chance(1, 6) {
            out.push(' ');
        }
        out.push(script.random_char(rng));
    }
    out
}

#[derive(Clone, Debug)]
pub struct MixedSpec {
    pub primary: ScriptKind,
    pub secondary: ScriptKind,
    /// Number of secondary-script characters injected among primary ones.
    pub secondary_count: usize,
}

/// Generate a primary-script fragment and overwrite `secondary_count`
/// positions with secondary script characters.
pub fn gen_mixed_fragment(rng: &mut Rng, spec: &MixedSpec) -> String {
    let chars: Vec<char> = gen_pure_fragment(rng, spec.primary).chars().collect();
    let mut mixed = chars.clone();
    let slots: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, ch)| **ch != ' ')
        .map(|(i, _)| i)
        .collect();
    if !slots.is_empty() && spec.secondary_count > 0 {
        for i in 0..spec.secondary_count {
            let slot = slots[i % slots.len()];
            mixed[slot] = spec.secondary.random_char(rng);
        }
    }
    mixed.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Noise decoration: punctuation, digits, line breaks and leading/trailing
// whitespace. All ASCII stop characters, so script counters stay unchanged.
// ---------------------------------------------------------------------------

const PUNCTUATION: [char; 10] = ['.', ',', '!', '?', ';', ':', '-', '"', '(', ')'];
const LINE_BREAKS: [&str; 3] = ["\n", "\r\n", "\r"];
const LEAD_TRAIL_WS: [&str; 4] = [" ", "\t", "  \n", "\t\r\n"];

pub struct Noise {
    pub text: String,
}

pub fn decorate_with_noise(rng: &mut Rng, text: &str) -> Noise {
    let mut out = String::new();

    // Leading whitespace
    if rng.chance(2, 3) {
        out.push_str(rng.pick(&LEAD_TRAIL_WS));
    }

    for ch in text.chars() {
        out.push(ch);
        if ch != ' ' && rng.chance(1, 5) {
            match rng.below(4) {
                0 => out.push(*rng.pick(&PUNCTUATION)),
                1 => out.push(char::from_digit(rng.below(10) as u32, 10).unwrap()),
                2 => out.push_str(rng.pick(&LINE_BREAKS)),
                _ => out.push(' '),
            }
        }
    }

    // Trailing whitespace
    if rng.chance(2, 3) {
        out.push_str(rng.pick(&LEAD_TRAIL_WS));
    }

    Noise { text: out }
}

// ---------------------------------------------------------------------------
// Filter configurations (built only through public Detector constructors)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum FilterCfg {
    None,
    Allow(Vec<Lang>),
    Deny(Vec<Lang>),
}

impl FilterCfg {
    pub fn label(&self) -> String {
        match self {
            FilterCfg::None => "none".to_string(),
            FilterCfg::Allow(langs) => format!("allow({})", lang_list(langs)),
            FilterCfg::Deny(langs) => format!("deny({})", lang_list(langs)),
        }
    }

    pub fn build_detector(&self, method: Method) -> Detector {
        let detector = match self {
            FilterCfg::None => Detector::new(),
            FilterCfg::Allow(langs) => Detector::with_allowlist(langs.clone()),
            FilterCfg::Deny(langs) => Detector::with_denylist(langs.clone()),
        };
        detector.set_method(method)
    }

    pub fn allows(&self, lang: Lang) -> bool {
        match self {
            FilterCfg::None => true,
            FilterCfg::Allow(list) => list.contains(&lang),
            FilterCfg::Deny(list) => !list.contains(&lang),
        }
    }
}

fn lang_list(langs: &[Lang]) -> String {
    langs
        .iter()
        .map(|l| format!("{l:?}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Pick up to `max` distinct members of `pool` at random, preserving the
/// RNG-derived order (the matrix tests also assert order independence with
/// explicitly reversed lists).
pub fn random_subset(rng: &mut Rng, pool: &[Lang], max: usize) -> Vec<Lang> {
    let mut indexed: Vec<(usize, Lang)> = pool.iter().enumerate().map(|(i, l)| (i, *l)).collect();
    let count = rng.range(0, max.min(pool.len()));
    // Fisher-Yates partial shuffle on indices
    for i in 0..count {
        let j = i + rng.below(indexed.len() - i);
        indexed.swap(i, j);
    }
    indexed.truncate(count);
    indexed.sort_unstable_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, l)| l).collect()
}

/// Deterministically generate one Allow and one Deny filter of a given size
/// plus their reversed twins.
pub fn sized_filters(pool: &[Lang], size: usize) -> Vec<FilterCfg> {
    let members: Vec<Lang> = pool.iter().take(size).copied().collect();
    let mut reversed = members.clone();
    reversed.reverse();
    vec![
        FilterCfg::Allow(members.clone()),
        FilterCfg::Allow(reversed),
        FilterCfg::Deny(members.clone()),
        FilterCfg::Deny(members),
    ]
}

// ---------------------------------------------------------------------------
// Invariant checks against the public contract
// ---------------------------------------------------------------------------

/// Contract every detection result must satisfy.
/// - repeat detection is fully deterministic (Info: PartialEq + same script);
/// - confidence is finite and inside the publicly documented 0..=1 range.
pub fn assert_core_invariants(detector: &Detector, text: &str) -> Option<Info> {
    let info1 = detector.detect(text);
    let info2 = detector.detect(text);

    match (&info1, &info2) {
        (Some(a), Some(b)) => {
            assert_eq!(
                (a.lang(), a.script(), a.confidence().to_bits()),
                (b.lang(), b.script(), b.confidence().to_bits()),
                "invariant violated: repeat detection is not deterministic"
            );
            let conf = a.confidence();
            assert!(
                conf.is_finite(),
                "invariant violated: confidence is not finite ({conf})"
            );
            assert!(
                (0.0..=1.0).contains(&conf),
                "invariant violated: confidence {conf} is outside the public 0..=1 range"
            );
        }
        (None, None) => {}
        _ => panic!("invariant violated: repeat detection switched Some/None"),
    }
    info1
}

/// whitelist results must come exclusively from the allowed set;
/// denylist must never return an excluded language.
pub fn assert_filter_respected(cfg: &FilterCfg, info: Option<&Info>) {
    if let Some(info) = info {
        assert!(
            cfg.allows(info.lang()),
            "invariant violated: filter {} returned non-member {:?}",
            cfg.label(),
            info.lang()
        );
    }
}

/// A one-member allowlist locks language and, per current public contract,
/// yields confidence 1.0 whenever script detection succeeds.
pub fn assert_single_candidate_contract(detector: &Detector, lang: Lang, text: &str) {
    if whatlang::detect_script(text).is_none() {
        return;
    }
    let info = detector
        .detect(text)
        .expect("invariant violated: single candidate returned None for script-bearing text");
    assert_eq!(
        info.lang(),
        lang,
        "invariant violated: single candidate language"
    );
    assert_eq!(
        info.confidence(),
        1.0,
        "invariant violated: single candidate confidence must be 1.0"
    );
}

// ---------------------------------------------------------------------------
// Candidate ranking derived purely from public API.
//
// To obtain the runner-up without any private backdoor, each language of the
// detected script's language set is scored on its own single-candidate
// detector (the public single-candidate contract fixes its value at 1.0), so
// instead the runner-up is computed by re-running detection with the winner
// denied. The winner's own confidence is the unfiltered one. This mirrors the
// "top two + margin" view needed by the profile tests while asserting only
// public behavior.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Ranked {
    pub winner: Lang,
    pub winner_conf: f64,
    pub runner_up: Option<Lang>,
    pub runner_up_conf: Option<f64>,
    pub margin: Option<f64>,
}

pub fn rank_top_two(method: Method, cfg: &FilterCfg, text: &str) -> Option<Ranked> {
    let detector = cfg.build_detector(method);
    let info = detector.detect(text)?;
    let winner = info.lang();
    let winner_conf = info.confidence();

    // Build a deny list on top of the current filter semantics: winner plus
    // every non-allowed language.
    let denied: Vec<Lang> = {
        let script_langs = info.script().langs().to_vec();
        let mut list = Vec::new();
        if let FilterCfg::Deny(extra) = cfg {
            list.extend(extra.iter().copied());
        }
        for lang in Lang::all() {
            if (!cfg.allows(*lang) || *lang == winner)
                && !list.contains(lang)
                && script_langs.contains(lang)
            {
                list.push(*lang);
            }
        }
        list
    };

    let runner = Detector::with_denylist(denied)
        .set_method(method)
        .detect(text);
    let runner_up = runner.as_ref().map(|i| i.lang());
    let runner_up_conf = runner.as_ref().map(|i| i.confidence());
    let margin = runner_up_conf.map(|c| winner_conf - c);

    Some(Ranked {
        winner,
        winner_conf,
        runner_up,
        runner_up_conf,
        margin,
    })
}

// ---------------------------------------------------------------------------
// Delta debugging style minimization over code points
// ---------------------------------------------------------------------------

/// Reduce `text` to a locally minimal code point sequence that still makes
/// `property` fail. Characters are removed; script identity of the original
/// sample may change while shrinking, but every intermediate text is a
/// subsequence of the failing input.
pub fn minimize<F>(mut text: String, mut property: F) -> String
where
    F: FnMut(&str) -> bool, // true => holds (pass), false => fails
{
    let mut chunk_count = 2usize.max(text.chars().count());
    let mut passes = 0;
    while chunk_count > 1 && passes < 12 {
        passes += 1;
        let chunk_size = text.chars().count().div_ceil(chunk_count);
        if chunk_size == 0 {
            break;
        }
        let mut reduced = false;
        let mut start = 0usize;
        while start < text.chars().count() {
            let chars: Vec<char> = text.chars().collect();
            let end = (start + chunk_size).min(chars.len());
            let candidate: String = chars[..start].iter().chain(chars[end..].iter()).collect();
            if !property(&candidate) {
                text = candidate;
                reduced = true;
                break;
            }
            start = end;
        }
        if !reduced {
            chunk_count = (chunk_count - 1).max(1);
        } else {
            chunk_count = 2usize.max(text.chars().count());
        }
    }
    text
}

pub fn codepoints(text: &str) -> String {
    text.chars()
        .map(|ch| format!("U+{:04X}", ch as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Failure report and fixed-seed runner
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CaseCtx {
    pub seed: u64,
    pub iteration: usize,
    pub filter: FilterCfg,
    pub method: Method,
}

impl CaseCtx {
    pub fn report(&self, text: &str, extra: &str) -> String {
        format!(
            "seed={} iteration={} method={:?} filter={} codepoints=[{}] {}\ntext={:?}",
            self.seed,
            self.iteration,
            self.method,
            self.filter.label(),
            codepoints(text),
            extra,
            text
        )
    }
}

/// Build one generated case. The returned closure is the property:
/// `Ok(())` when the invariant holds, `Err(invariant description)` otherwise.
pub type Property = Box<dyn Fn(&str) -> Result<(), String>>;

/// Run `make` over the fixed seed list. On failure the generated text is
/// minimized (classic delta-debugging over code points) and the assertion
/// message carries seed, minimized codepoints, filter config, method and the
/// current candidate ranking.
pub fn run_fixed_seeds<F>(seeds: &[u64], cases_per_seed: usize, mut make: F)
where
    F: FnMut(&mut Rng, &CaseCtx) -> (String, Property),
{
    for &seed in seeds {
        for iteration in 0..cases_per_seed {
            let mut rng = Rng::new(seed.wrapping_add(iteration as u64 * 0x9E37));
            let ctx = CaseCtx {
                seed,
                iteration,
                filter: FilterCfg::None,
                method: Method::Combined,
            };
            let (text, property) = make(&mut rng, &ctx);
            if let Err(msg) = property(&text) {
                let minimized = minimize(text.clone(), |t| property(t).is_ok());
                let ranked = rank_top_two(ctx.method, &ctx.filter, &minimized)
                    .map(|r| {
                        format!(
                            "ranking=winner:{:?}({}) runner_up:{:?}({:?}) margin:{:?}",
                            r.winner, r.winner_conf, r.runner_up, r.runner_up_conf, r.margin
                        )
                    })
                    .unwrap_or_else(|| "ranking=<no candidates>".to_string());
                panic!(
                    "{}\nviolated invariant: {msg}",
                    ctx.report(&minimized, &ranked)
                );
            }
        }
    }
}

/// Same as [`run_fixed_seeds`], but the case builder decides the filter and
/// method itself (filter matrix tests).
pub fn run_fixed_seeds_cfg<F>(seeds: &[u64], cases_per_seed: usize, mut make: F)
where
    F: FnMut(&mut Rng, &CaseCtx) -> (CaseCtx, String, Property),
{
    for &seed in seeds {
        for iteration in 0..cases_per_seed {
            let mut rng = Rng::new(seed.wrapping_add(iteration as u64 * 0x9E37));
            let base = CaseCtx {
                seed,
                iteration,
                filter: FilterCfg::None,
                method: Method::Combined,
            };
            let (ctx, text, property) = make(&mut rng, &base);
            if let Err(msg) = property(&text) {
                let minimized = minimize(text.clone(), |t| property(t).is_ok());
                let ranked = rank_top_two(ctx.method, &ctx.filter, &minimized)
                    .map(|r| {
                        format!(
                            "ranking=winner:{:?}({}) runner_up:{:?}({:?}) margin:{:?}",
                            r.winner, r.winner_conf, r.runner_up, r.runner_up_conf, r.margin
                        )
                    })
                    .unwrap_or_else(|| "ranking=<no candidates>".to_string());
                panic!(
                    "{}\nviolated invariant: {msg}",
                    ctx.report(&minimized, &ranked)
                );
            }
        }
    }
}

/// Names of every named property case. Printed once per test-binary run so
/// that `cargo test --quiet properties` still shows which cases ran
/// (libtest's `--quiet` mode otherwise only prints failures and the summary).
pub static PROPERTY_CASE_NAMES: &[&str] = &[
    "pure::properties_pure_latin_fragment_invariants",
    "pure::properties_pure_cyrillic_fragment_invariants",
    "pure::properties_pure_arabic_fragment_invariants",
    "pure::properties_pure_devanagari_fragment_invariants",
    "pure::properties_pure_hebrew_fragment_invariants",
    "pure::properties_pure_han_fragment_invariants",
    "pure::properties_pure_hiragana_fragment_invariants",
    "pure::properties_pure_katakana_fragment_invariants",
    "scripts_noise::properties_noise_keeps_latin_script",
    "scripts_noise::properties_noise_keeps_cyrillic_script",
    "scripts_noise::properties_noise_keeps_arabic_script",
    "scripts_noise::properties_noise_keeps_devanagari_script",
    "scripts_noise::properties_noise_keeps_hebrew_script",
    "scripts_noise::properties_noise_keeps_han_script",
    "mixed::properties_mixed_latin_cyrillic_invariants",
    "mixed::properties_mixed_cyrillic_latin_invariants",
    "mixed::properties_mixed_arabic_latin_invariants",
    "mixed::properties_mixed_devanagari_latin_invariants",
    "mixed::properties_mixed_hebrew_latin_invariants",
    "mixed::properties_mixed_han_kana_invariants",
    "filters::properties_filters_random_allowlist_membership",
    "filters::properties_filters_random_denylist_exclusion",
    "filters::properties_filters_matrix_sizes_and_member_order",
    "filters::properties_filters_empty_allowlist_contract",
    "filters::properties_filters_single_candidate_contract",
    "filters::properties_filters_denylist_all_family_members_contract",
    "filters::properties_mut_target_denylist_applied_before_ranking",
    "special_branches::properties_special_branches_mandarin_pure_han",
    "special_branches::properties_special_branches_kana_threshold_boundaries",
    "special_branches::properties_special_branches_kana_threshold_confidence_levels",
    "special_branches::properties_special_branches_allowlist_cmn_or_jpn",
    "special_branches::properties_special_branches_denylist_current_contract",
    "special_branches::properties_special_branches_random_han_kana_filter_invariants",
    "profile_pairs::properties_profile_pair_spa_por_records_top_two_and_margin",
    "profile_pairs::properties_profile_pair_rus_ukr_records_top_two_and_margin",
    "profile_pairs::properties_profile_pair_ara_pes_records_top_two_and_margin",
    "profile_pairs::properties_mut_target_confidence_normalization_range_trigram",
    "generator::properties_generator_sequence_is_replayable_in_order",
    "generator::properties_generator_seed_override_env",
];
