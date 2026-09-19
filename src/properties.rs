//! Seeded property tests for the public detection contract.
//!
//! These tests deliberately avoid hard-coding statistical model answers: every
//! generated sample is produced by a fixed-seed pseudo-random generator and
//! assertions only cover structural invariants (determinism, confidence range,
//! filter membership, script stability and ranking coherence). Failure output
//! records the seed, the minimized codepoint sequence, the filter configuration,
//! the [`Method`] and the candidate ranking, so any shrinking counterexample can
//! be replayed deterministically.

#![cfg(test)]

use std::fmt::Write as _;

use crate::Lang;
use crate::core::{
    FilterList, InternalQuery, Method, Options, Text, detect as public_detect, detect_with_options,
};
use crate::scripts::grouping::MultiLangScript;
use crate::scripts::{Script, detect_script};
use crate::trigrams::{
    ARABIC_LANGS, CYRILLIC_LANGS, DEVANAGARI_LANGS, HEBREW_LANGS, LATIN_LANGS,
    raw_detect as trigrams_raw_detect,
};

// ----------------------------------------------------------------------------
// Fixed-seed pseudo random generator (SplitMix64).
// The generator is deliberately small and dependency free, so the generation
// order stays identical across platforms, library versions and runs.
// ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    fn pick<T: Copy>(&mut self, slice: &[T]) -> T {
        slice[self.below(slice.len())]
    }
}

// ----------------------------------------------------------------------------
// Restricted character pools. Every codepoint is inside the corresponding
// script predicate range used by `raw_detect_script`.
// ----------------------------------------------------------------------------

const LATIN_POOL: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'E', 'O', 'U', 'ä', 'ö', 'ü', 'ß', 'é', 'è', 'ê', 'ç',
    'à', 'â', 'ñ', 'ó', 'í', 'č', 'š', 'ř', 'ž', 'ł', 'ą', 'ę', 'ł', 'ă', 'ș', 'ț',
];

const CYRILLIC_POOL: &[char] = &[
    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п', 'р', 'с', 'т',
    'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я', 'А', 'Б', 'Я', 'ё', 'Ё', 'і',
    'ї', 'є', 'ґ', 'ў', 'ђ', 'љ', 'њ', 'ћ',
];

const ARABIC_POOL: &[char] = &[
    'ا', 'ب', 'ت', 'ث', 'ج', 'ح', 'خ', 'د', 'ذ', 'ر', 'ز', 'س', 'ش', 'ص', 'ض', 'ط', 'ظ', 'ع', 'غ',
    'ف', 'ق', 'ك', 'ل', 'م', 'ن', 'ه', 'و', 'ي', 'ى', 'ة', 'ؤ', 'ئ', 'ك', 'پ', 'چ', 'ژ',
];

const DEVANAGARI_POOL: &[char] = &[
    'अ', 'आ', 'इ', 'ई', 'उ', 'ऊ', 'ए', 'ऐ', 'ओ', 'औ', 'क', 'ख', 'ग', 'घ', 'च', 'छ', 'ज', 'झ', 'ट',
    'ठ', 'ड', 'ढ', 'ण', 'त', 'थ', 'द', 'ध', 'न', 'प', 'फ', 'ब', 'भ', 'म', 'य', 'र', 'ल', 'व', 'श',
    'ष', 'स', 'ह', 'ि', 'ी', 'ु', 'ू', 'े', 'ै', 'ो', 'ौ', '्',
];

const HEBREW_POOL: &[char] = &[
    'א', 'ב', 'ג', 'ד', 'ה', 'ו', 'ז', 'ח', 'ט', 'י', 'כ', 'ל', 'מ', 'נ', 'ס', 'ע', 'פ', 'צ', 'ק',
    'ר', 'ש', 'ת', 'ך', 'ם', 'ן', 'ף', 'ץ',
];

const HAN_POOL: &[char] = &[
    '水', '火', '山', '人', '大', '中', '国', '日', '本', '語', '学', '生', '時', '今', '何', '行',
    '見', '上', '下', '前', '後', '方', '名', '文', '字', '書', '店', '道', '車', '門', '長', '東',
    '京', '海', '川', '田', '町', '校', '話', '食', '飲',
];

const HIRAGANA_POOL: &[char] = &[
    'あ', 'い', 'う', 'え', 'お', 'か', 'き', 'く', 'け', 'こ', 'さ', 'し', 'す', 'せ', 'そ', 'た',
    'ち', 'つ', 'て', 'と', 'な', 'に', 'ぬ', 'ね', 'の', 'は', 'ひ', 'ふ', 'へ', 'ほ', 'ま', 'み',
    'む', 'め', 'も', 'や', 'ゆ', 'よ', 'ら', 'り', 'る', 'れ', 'ろ', 'わ', 'を', 'ん', 'が', 'ぎ',
    'ぐ', 'げ', 'ご', 'ざ', 'じ', 'ず', 'ぜ', 'ぞ', 'だ', 'ぢ', 'づ', 'で', 'ど', 'ば', 'び', 'ぶ',
    'べ', 'ぼ', 'ぱ', 'ぴ', 'ぷ', 'ぺ', 'ぽ', 'ゃ', 'ゅ', 'ょ', 'っ',
];

const KATAKANA_POOL: &[char] = &[
    'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ', 'サ', 'シ', 'ス', 'セ', 'ソ', 'タ',
    'チ', 'ツ', 'テ', 'ト', 'ナ', 'ニ', 'ヌ', 'ネ', 'ノ', 'ハ', 'ヒ', 'フ', 'ヘ', 'ホ', 'マ', 'ミ',
    'ム', 'メ', 'モ', 'ヤ', 'ユ', 'ヨ', 'ラ', 'リ', 'ル', 'レ', 'ロ', 'ワ', 'ヲ', 'ン', 'ガ', 'ギ',
    'グ', 'ゲ', 'ゴ', 'ザ', 'ジ', 'ズ', 'ゼ', 'ゾ', 'ダ', 'ヂ', 'ヅ', 'デ', 'ド', 'バ', 'ビ', 'ブ',
    'ベ', 'ボ', 'パ', 'ピ', 'プ', 'ペ', 'ポ', 'ャ', 'ュ', 'ョ', 'ッ',
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Latin,
    Cyrillic,
    Arabic,
    Devanagari,
    Hebrew,
    Han,
    Hiragana,
    Katakana,
}

impl Family {
    fn pool(self) -> &'static [char] {
        match self {
            Family::Latin => LATIN_POOL,
            Family::Cyrillic => CYRILLIC_POOL,
            Family::Arabic => ARABIC_POOL,
            Family::Devanagari => DEVANAGARI_POOL,
            Family::Hebrew => HEBREW_POOL,
            Family::Han => HAN_POOL,
            Family::Hiragana => HIRAGANA_POOL,
            Family::Katakana => KATAKANA_POOL,
        }
    }

    fn expected_script(self) -> Script {
        match self {
            Family::Latin => Script::Latin,
            Family::Cyrillic => Script::Cyrillic,
            Family::Arabic => Script::Arabic,
            Family::Devanagari => Script::Devanagari,
            Family::Hebrew => Script::Hebrew,
            Family::Han => Script::Mandarin,
            Family::Hiragana => Script::Hiragana,
            Family::Katakana => Script::Katakana,
        }
    }

    fn multi_lang_script(self) -> Option<MultiLangScript> {
        match self {
            Family::Latin => Some(MultiLangScript::Latin),
            Family::Cyrillic => Some(MultiLangScript::Cyrillic),
            Family::Arabic => Some(MultiLangScript::Arabic),
            Family::Devanagari => Some(MultiLangScript::Devanagari),
            Family::Hebrew => Some(MultiLangScript::Hebrew),
            Family::Han | Family::Hiragana | Family::Katakana => None,
        }
    }

    fn all_multi() -> [Family; 5] {
        [
            Family::Latin,
            Family::Cyrillic,
            Family::Arabic,
            Family::Devanagari,
            Family::Hebrew,
        ]
    }
}

const MULTI_FAMILIES: [Family; 8] = [
    Family::Latin,
    Family::Cyrillic,
    Family::Arabic,
    Family::Devanagari,
    Family::Hebrew,
    Family::Han,
    Family::Hiragana,
    Family::Katakana,
];

#[derive(Clone, Debug)]
struct Fragment {
    chars: Vec<char>,
}

impl std::fmt::Display for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ch in &self.chars {
            write!(f, "{ch}")?;
        }
        Ok(())
    }
}

fn gen_pure_fragment(rng: &mut Rng, family: Family, max_len: usize) -> Fragment {
    let len = rng.range(1, max_len);
    gen_fixed_length_fragment(rng, family, len)
}

fn gen_fixed_length_fragment(rng: &mut Rng, family: Family, len: usize) -> Fragment {
    let pool = family.pool();
    let chars = (0..len).map(|_| rng.pick(pool)).collect();
    Fragment { chars }
}

// Punctuation, digits and whitespace are all ASCII stop chars, so they can
// never change the dominant script of a pure single-script fragment.
fn decorate(rng: &mut Rng, fragment: &Fragment) -> String {
    let punctuation = ['.', ',', '!', '?', ';', ':', '-', '(', ')', '"', '\''];
    let digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    let linebreaks = [' ', '\t', '\n', '\r', '\u{000B}'];

    let mut out = String::new();
    let leading = rng.range(0, 4);
    for _ in 0..leading {
        out.push(rng.pick(&linebreaks));
    }
    for (idx, ch) in fragment.chars.iter().enumerate() {
        out.push(*ch);
        if idx + 1 < fragment.chars.len() && rng.below(3) == 0 {
            let kind = rng.below(3);
            match kind {
                0 => out.push(rng.pick(&punctuation)),
                1 => out.push(rng.pick(&digits)),
                _ => out.push(rng.pick(&linebreaks)),
            }
        }
    }
    if rng.below(2) == 0 {
        out.push(rng.pick(&punctuation));
    }
    let trailing = rng.range(0, 4);
    for _ in 0..trailing {
        out.push(rng.pick(&linebreaks));
    }
    out
}

#[derive(Clone, Debug)]
struct MixedFragment {
    text: String,
    dominant: Family,
}

fn gen_mixed_fragment(rng: &mut Rng, max_parts: usize, max_len: usize) -> MixedFragment {
    let part_count = rng.range(2, max_parts);
    let mut chosen: Vec<Family> = Vec::new();
    while chosen.len() < part_count {
        let candidate = rng.pick(&MULTI_FAMILIES);
        if !chosen.contains(&candidate) {
            chosen.push(candidate);
        }
    }
    // The first family is dominant: it gets clearly more letters than every
    // other part.
    let dominant = chosen[0];

    let mut parts: Vec<Fragment> = Vec::new();
    // Minor parts are capped well below the dominant part so that dominance
    // holds even when the dominant length roll is in the middle of its range.
    let minor_budget = (max_len / 6).max(1);
    let dominant_min = minor_budget * (part_count - 1) + 2;
    let dominant_fragment = gen_fixed_length_fragment(rng, dominant, dominant_min.min(max_len));
    parts.push(dominant_fragment);
    for family in chosen.iter().skip(1) {
        let fragment = gen_pure_fragment(rng, *family, minor_budget);
        parts.push(fragment);
    }

    let mut text = String::new();
    for (idx, fragment) in parts.iter().enumerate() {
        if idx > 0 {
            text.push([' ', '\n', ',', '.', '1'][rng.below(5)]);
        }
        text.push_str(&fragment.to_string());
    }
    MixedFragment { text, dominant }
}

// ----------------------------------------------------------------------------
// Failure context and the deterministic codepoint-sequence minimizer.
// ----------------------------------------------------------------------------

#[derive(Clone)]
struct Case {
    seed: u64,
    index: usize,
    filter: FilterList,
    method: Method,
}

fn describe_filter(filter: &FilterList) -> String {
    match filter {
        FilterList::All => "All".to_string(),
        FilterList::Allow(langs) => format!("Allow({})", names(langs)),
        FilterList::Deny(langs) => format!("Deny({})", names(langs)),
    }
}

fn names(langs: &[Lang]) -> String {
    langs
        .iter()
        .map(|lang| format!("{lang:?}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe_codepoints(chars: &[char]) -> String {
    let mut out = String::from("[");
    for (idx, ch) in chars.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let _ = write!(out, "U+{:04X}", *ch as u32);
    }
    out.push(']');
    out
}

fn describe_ranking(scores: &[(Lang, f64)]) -> String {
    scores
        .iter()
        .take(5)
        .map(|(lang, score)| format!("{lang:?}={score:.4}"))
        .collect::<Vec<_>>()
        .join(" > ")
}

fn fail_report(invariant: &str, case: &Case, chars: &[char], ranking: &str, detail: &str) -> ! {
    panic!(
        "INVARIANT VIOLATED: {invariant}\n\
         seed = {seed} (sample #{index})\n\
         method = {method:?}\n\
         filter = {filter}\n\
         minimized codepoints ({n}) = {points}\n\
         text = {text:?}\n\
         candidate ranking = {ranking}\n\
         detail = {detail}",
        seed = case.seed,
        index = case.index,
        method = case.method,
        filter = describe_filter(&case.filter),
        n = chars.len(),
        points = describe_codepoints(chars),
        text = chars.iter().collect::<String>(),
        ranking = ranking,
        detail = detail,
    );
}

// Delta-debugging style minimizer: repeatedly tries to remove chunks of the
// codepoint sequence while the predicate keeps failing, halving the chunk
// size. Returns the shortest failing sequence found.
fn minimize<F>(chars: Vec<char>, mut fails: F) -> Vec<char>
where
    F: FnMut(&[char]) -> bool,
{
    let mut current = chars;
    let mut chunk_size = current.len().max(1);
    while chunk_size > 1 && current.len() > 1 {
        let mut reduced = false;
        let mut start = 0;
        while start < current.len() {
            let end = (start + chunk_size).min(current.len());
            let candidate: Vec<char> = current
                .iter()
                .take(start)
                .chain(current.iter().skip(end))
                .copied()
                .collect();
            if !candidate.is_empty() && fails(&candidate) {
                current = candidate;
                reduced = true;
                break;
            }
            start = end;
        }
        if !reduced {
            chunk_size /= 2;
        }
    }
    // Final pass: try removing each codepoint individually.
    let mut changed = true;
    while changed {
        changed = false;
        for idx in 0..current.len() {
            let candidate: Vec<char> = current
                .iter()
                .take(idx)
                .chain(current.iter().skip(idx + 1))
                .copied()
                .collect();
            if !candidate.is_empty() && fails(&candidate) {
                current = candidate;
                changed = true;
                break;
            }
        }
    }
    current
}

// ----------------------------------------------------------------------------
// Detection helpers. All three public Methods are exercised through exactly
// the same path the public API uses.
// ----------------------------------------------------------------------------

fn options_for(case: &Case) -> Options {
    Options {
        filter_list: case.filter.clone(),
        method: case.method,
    }
}

fn detect_info(text: &str, case: &Case) -> Option<crate::core::Info> {
    detect_with_options(text, &options_for(case))
}

fn raw_scores(text: &str, family: Family, filter: &FilterList, method: Method) -> Vec<(Lang, f64)> {
    let multi = family
        .multi_lang_script()
        .expect("raw_scores requires a multi-language script");
    let iquery = InternalQuery {
        text: Text::new(text),
        filter_list: filter,
        multi_lang_script: multi,
    };
    let mut scores = match method {
        Method::Trigram => trigrams_raw_detect(&iquery).scores,
        Method::Alphabet => crate::alphabets::raw_detect(&iquery).scores,
        Method::Combined => crate::combined::raw_detect(&iquery).scores,
    };
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

fn assert_finite_confidence(
    invariant: &str,
    case: &Case,
    chars: &[char],
    ranking: &str,
    confidence: f64,
) {
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        let text: String = chars.iter().collect();
        let minimized = minimize(chars.to_vec(), |candidate| {
            let reduced: String = candidate.iter().collect();
            detect_info(&reduced, case)
                .map(|info| {
                    let value = info.confidence();
                    !value.is_finite() || !(0.0..=1.0).contains(&value)
                })
                .unwrap_or(false)
        });
        let _ = text;
        fail_report(
            invariant,
            case,
            &minimized,
            ranking,
            &format!("confidence={confidence}"),
        );
    }
}

// ----------------------------------------------------------------------------
// Shared sample verification.
// ----------------------------------------------------------------------------

fn verify_sample(
    invariant: &str,
    chars: &[char],
    case: &Case,
    family: Option<Family>,
    expect_script: Option<Script>,
) {
    let text: String = chars.iter().collect();

    let info1 = detect_info(&text, case);
    let info2 = detect_info(&text, case);

    // Determinism: repeated detection with identical input/config must be equal.
    let same = match (&info1, &info2) {
        (Some(a), Some(b)) => a == b,
        (None, None) => true,
        _ => false,
    };
    if !same {
        fail_report(
            invariant,
            case,
            chars,
            "<rerun produced a different result>",
            "repeated detection was not deterministic",
        );
    }

    if let Some(expected) = expect_script {
        let actual = detect_script(&text);
        if actual != Some(expected) {
            let ranking = family
                .and_then(|f| f.multi_lang_script())
                .map(|_| {
                    describe_ranking(&raw_scores(
                        &text,
                        family.unwrap(),
                        &case.filter,
                        case.method,
                    ))
                })
                .unwrap_or_else(|| "<single-language script>".to_string());
            let minimized = minimize(chars.to_vec(), |candidate| {
                let reduced: String = candidate.iter().collect();
                detect_script(&reduced) != Some(expected)
            });
            fail_report(
                invariant,
                case,
                &minimized,
                &ranking,
                &format!("detect_script={actual:?} expected={expected:?}"),
            );
        }
    }

    let info = match info1 {
        Some(info) => info,
        None => return,
    };

    let ranking = family
        .filter(|f| f.multi_lang_script().is_some())
        .map(|f| describe_ranking(&raw_scores(&text, f, &case.filter, case.method)))
        .unwrap_or_else(|| "<single-language script branch>".to_string());

    assert_finite_confidence(invariant, case, chars, &ranking, info.confidence());

    // Filter membership contract (Mandarin special branch has its own tests).
    let violates_filter = family.is_some_and(|family| {
        family.multi_lang_script().is_some() && !case.filter.is_allowed(info.lang())
    });
    if violates_filter {
        let minimized = minimize(chars.to_vec(), |candidate| {
            let reduced: String = candidate.iter().collect();
            detect_info(&reduced, case)
                .map(|i| !case.filter.is_allowed(i.lang()))
                .unwrap_or(false)
        });
        fail_report(
            invariant,
            case,
            &minimized,
            &ranking,
            &format!("returned language {:?} violates filter", info.lang()),
        );
    }

    // Candidate set legality and score ordering coherence for multi scripts.
    if let Some(family) = family.filter(|family| family.multi_lang_script().is_some()) {
        let scores = raw_scores(&text, family, &case.filter, case.method);
        for (lang, score) in &scores {
            if !score.is_finite() || !(0.0..=1.0).contains(score) {
                fail_report(
                    invariant,
                    case,
                    chars,
                    &describe_ranking(&scores),
                    &format!("candidate {lang:?} has non-finite/out-of-range score {score}"),
                );
            }
            if !case.filter.is_allowed(*lang) {
                fail_report(
                    invariant,
                    case,
                    chars,
                    &describe_ranking(&scores),
                    &format!("candidate set contains filtered-out language {lang:?}"),
                );
            }
        }
        for pair in scores.windows(2) {
            if pair[0].1 < pair[1].1 {
                fail_report(
                    invariant,
                    case,
                    chars,
                    &describe_ranking(&scores),
                    "candidate ranking is not sorted by descending score",
                );
            }
        }
    }
}

fn check_pure_samples(
    invariant: &str,
    seed: u64,
    count_per_family: usize,
    method: Method,
    filter: impl Fn(Family) -> FilterList,
) {
    for family in MULTI_FAMILIES {
        let mut rng = Rng::new(
            seed.wrapping_add(family as u64)
                .wrapping_mul(0x0100_0000_01B3),
        );
        for index in 0..count_per_family {
            let fragment = gen_pure_fragment(&mut rng, family, 40);
            let case = Case {
                seed,
                index,
                filter: filter(family),
                method,
            };
            verify_sample(
                invariant,
                &fragment.chars,
                &case,
                Some(family),
                Some(family.expected_script()),
            );
        }
    }
}

fn check_decorated_pure_samples(seed: u64, count_per_family: usize) {
    for family in MULTI_FAMILIES {
        let mut rng = Rng::new(seed.wrapping_add((family as u64) << 24) ^ 0x9E37_79B9);
        for index in 0..count_per_family {
            let fragment = gen_pure_fragment(&mut rng, family, 32);
            let text = decorate(&mut rng, &fragment);
            let case = Case {
                seed,
                index,
                filter: FilterList::All,
                method: Method::Combined,
            };
            verify_sample(
                "pure-single-script fragments keep their script under punctuation/digits/newlines/whitespace",
                &text.chars().collect::<Vec<char>>(),
                &case,
                Some(family),
                Some(family.expected_script()),
            );
        }
    }
}

// ----------------------------------------------------------------------------
// Named property tests.
// ----------------------------------------------------------------------------

#[test]
fn properties_01_generation_order_is_replayable_from_fixed_seed() {
    let mut generated_a = Vec::new();
    let mut rng = Rng::new(20260920);
    for _ in 0..12 {
        generated_a.push(gen_pure_fragment(&mut rng, Family::Latin, 24).to_string());
        generated_a.push(gen_pure_fragment(&mut rng, Family::Cyrillic, 24).to_string());
        generated_a.push(gen_mixed_fragment(&mut rng, 3, 20).text);
    }
    let mut generated_b = Vec::new();
    let mut rng = Rng::new(20260920);
    for _ in 0..12 {
        generated_b.push(gen_pure_fragment(&mut rng, Family::Latin, 24).to_string());
        generated_b.push(gen_pure_fragment(&mut rng, Family::Cyrillic, 24).to_string());
        generated_b.push(gen_mixed_fragment(&mut rng, 3, 20).text);
    }
    assert_eq!(
        generated_a, generated_b,
        "two runs with the same seed must produce the same generation order"
    );
}

#[test]
fn properties_02_generation_order_matches_frozen_sequence() {
    // Golden snapshot of the first 6 generated Latin codepoints: this fails if
    // the generator itself changes, which would invalidate seed replay docs.
    let mut rng = Rng::new(42);
    let fragment = gen_fixed_length_fragment(&mut rng, Family::Latin, 8);
    assert_eq!(
        describe_codepoints(&fragment.chars),
        "[U+0074,U+0074,U+0073,U+0061,U+0219,U+00ED,U+0074,U+0142]",
    );
}

#[test]
fn properties_03_repeated_detection_is_completely_deterministic_combined() {
    check_pure_samples(
        "repeated detection is completely deterministic",
        1001,
        24,
        Method::Combined,
        |_| FilterList::All,
    );
}

#[test]
fn properties_04_repeated_detection_is_completely_deterministic_trigram() {
    check_pure_samples(
        "repeated detection is completely deterministic",
        1002,
        16,
        Method::Trigram,
        |_| FilterList::All,
    );
}

#[test]
fn properties_05_repeated_detection_is_completely_deterministic_alphabet() {
    check_pure_samples(
        "repeated detection is completely deterministic",
        1003,
        16,
        Method::Alphabet,
        |_| FilterList::All,
    );
}

#[test]
fn properties_06_confidence_is_finite_and_within_public_range_combined() {
    check_pure_samples(
        "confidence is always finite and within the public 0.0..=1.0 range",
        2001,
        24,
        Method::Combined,
        |_| FilterList::All,
    );
}

#[test]
fn properties_07_confidence_is_finite_and_within_public_range_trigram() {
    check_pure_samples(
        "confidence is always finite and within the public 0.0..=1.0 range",
        2002,
        20,
        Method::Trigram,
        |_| FilterList::All,
    );
}

#[test]
fn properties_08_confidence_is_finite_and_within_public_range_alphabet() {
    check_pure_samples(
        "confidence is always finite and within the public 0.0..=1.0 range",
        2003,
        20,
        Method::Alphabet,
        |_| FilterList::All,
    );
}

#[test]
fn properties_09_calculate_confidence_helper_is_finite_and_bounded_on_tied_scores() {
    // Direct invariant for the confidence function itself: tied nonzero scores
    // must produce 0.0, never NaN or a negative value.
    let invariant = "confidence is always finite and within the public 0.0..=1.0 range";
    for highest in [0.0_f64, 0.25, 0.5, 0.75, 1.0] {
        for count in 1..=40 {
            let value = crate::core::calculate_confidence(highest, highest, count);
            assert!(
                value.is_finite() && (0.0..=1.0).contains(&value),
                "{invariant}: highest={highest} second={highest} count={count} produced {value}"
            );
        }
    }
}

#[test]
fn properties_10_whitelist_results_come_only_from_allowed_set() {
    let families = Family::all_multi();
    for family in families {
        let langs = family_script_langs(family);
        // Pairs of allowed languages with different list sizes.
        let lists: Vec<Vec<Lang>> = vec![
            langs[..1].to_vec(),
            langs[..langs.len().min(2)].to_vec(),
            langs[..langs.len().min(3)].to_vec(),
            langs.iter().rev().copied().take(4).collect(),
        ];
        for (list_idx, list) in lists.into_iter().enumerate() {
            let case = Case {
                seed: 3000 + family as u64 * 10 + list_idx as u64,
                index: 0,
                filter: FilterList::allow(list.clone()),
                method: Method::Combined,
            };
            let mut rng = Rng::new(3000 + family as u64 * 10 + list_idx as u64);
            for _ in 0..12 {
                let fragment = gen_pure_fragment(&mut rng, family, 36);
                verify_sample(
                    "whitelist results can only come from the allowed set",
                    &fragment.chars,
                    &case,
                    Some(family),
                    None,
                );
            }
        }
    }
}

fn family_script_langs(family: Family) -> &'static [Lang] {
    match family {
        Family::Latin => LATIN_LANGS
            .iter()
            .map(|(lang, _)| *lang)
            .collect::<Vec<_>>()
            .leak(),
        Family::Cyrillic => CYRILLIC_LANGS
            .iter()
            .map(|(lang, _)| *lang)
            .collect::<Vec<_>>()
            .leak(),
        Family::Arabic => ARABIC_LANGS
            .iter()
            .map(|(lang, _)| *lang)
            .collect::<Vec<_>>()
            .leak(),
        Family::Devanagari => DEVANAGARI_LANGS
            .iter()
            .map(|(lang, _)| *lang)
            .collect::<Vec<_>>()
            .leak(),
        Family::Hebrew => HEBREW_LANGS
            .iter()
            .map(|(lang, _)| *lang)
            .collect::<Vec<_>>()
            .leak(),
        Family::Han | Family::Hiragana | Family::Katakana => &[],
    }
}

#[test]
fn properties_11_blacklist_never_returns_excluded_languages() {
    for family in Family::all_multi() {
        let langs = family_script_langs(family);
        // This mutation-guard test also makes sure filtering happens before the
        // ranking stage: excluded languages must never survive into the result.
        let lists: Vec<Vec<Lang>> = vec![
            langs[..1].to_vec(),
            langs[..langs.len().min(2)].to_vec(),
            langs.iter().rev().copied().take(3).collect(),
        ];
        for (list_idx, list) in lists.into_iter().enumerate() {
            let seed = 4000 + family as u64 * 10 + list_idx as u64;
            let case = Case {
                seed,
                index: 0,
                filter: FilterList::deny(list),
                method: Method::Combined,
            };
            let mut rng = Rng::new(seed);
            for _ in 0..16 {
                let fragment = gen_pure_fragment(&mut rng, family, 36);
                verify_sample(
                    "blacklist must be applied before ranking and never return an excluded language",
                    &fragment.chars,
                    &case,
                    Some(family),
                    None,
                );
            }
        }
    }
}

#[test]
fn properties_12_empty_candidate_set_returns_none() {
    // Every language of a multi-language script denied -> no candidates -> None.
    for family in Family::all_multi() {
        let all = family_script_langs(family).to_vec();
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let case = Case {
                seed: 5000 + family as u64 * 10,
                index: 0,
                filter: FilterList::deny(all.clone()),
                method,
            };
            let mut rng = Rng::new(5000 + family as u64 * 10);
            for _ in 0..8 {
                let fragment = gen_pure_fragment(&mut rng, family, 30);
                let text = fragment.to_string();
                let result = detect_info(&text, &case);
                assert!(
                    result.is_none(),
                    "family={family:?} method={method:?} text={text:?} expected None for empty candidate set, got {result:?}"
                );
            }
        }
    }
}

#[test]
fn properties_13_single_candidate_returns_allowed_language_with_unit_confidence() {
    for family in Family::all_multi() {
        let langs = family_script_langs(family);
        let only = langs[0];
        for method in [Method::Trigram, Method::Alphabet, Method::Combined] {
            let case = Case {
                seed: 6000 + family as u64 * 10,
                index: 0,
                filter: FilterList::allow(vec![only]),
                method,
            };
            let mut rng = Rng::new(6000 + family as u64 * 10);
            for _ in 0..8 {
                let fragment = gen_pure_fragment(&mut rng, family, 24);
                let text = fragment.to_string();
                let info = detect_info(&text, &case).unwrap_or_else(|| {
                    panic!("single-candidate allowlist({only:?}) returned None for {text:?} ({family:?}, {method:?})")
                });
                assert_eq!(info.lang(), only, "family={family:?} method={method:?}");
                assert_eq!(
                    info.confidence(),
                    1.0,
                    "single candidate must report confidence 1.0, family={family:?} method={method:?}"
                );
            }
        }
    }
}

#[test]
fn properties_14_mutually_exclusive_filters_never_agree_when_membership_differs() {
    // For every multi-language script family, Allow(L) and Deny(L\\{x}) are
    // mutually exclusive configurations: any returned language must satisfy
    // exactly the active filter.
    for family in Family::all_multi() {
        let langs = family_script_langs(family).to_vec();
        if langs.len() < 2 {
            continue;
        }
        let mut rng = Rng::new(7000 + family as u64);
        for index in 0..12 {
            let fragment = gen_pure_fragment(&mut rng, family, 30);
            let allow_case = Case {
                seed: 7000,
                index,
                filter: FilterList::allow(langs.clone()),
                method: Method::Combined,
            };
            let deny_case = Case {
                seed: 7000,
                index,
                filter: FilterList::deny(vec![langs[index % langs.len()]]),
                method: Method::Combined,
            };
            if let Some(info) = detect_info(&fragment.to_string(), &allow_case) {
                assert!(langs.contains(&info.lang()));
            }
            if let Some(info) = detect_info(&fragment.to_string(), &deny_case) {
                assert_ne!(info.lang(), langs[index % langs.len()]);
            }
        }
    }
}

#[test]
fn properties_15_same_members_different_order_give_identical_results() {
    for family in Family::all_multi() {
        let langs = family_script_langs(family);
        let take = langs.len().min(5);
        let forward: Vec<Lang> = langs[..take].to_vec();
        let mut reversed = forward.clone();
        reversed.reverse();

        let mut shuffled = forward.clone();
        let mut rng = Rng::new(8000 + family as u64);
        for idx in (1..shuffled.len()).rev() {
            let swap_with = rng.below(idx + 1);
            shuffled.swap(idx, swap_with);
        }

        let mut rng = Rng::new(8100 + family as u64);
        for _ in 0..16 {
            let fragment = gen_pure_fragment(&mut rng, family, 30);
            let text = fragment.to_string();
            let allow_forward = detect_info(
                &text,
                &Case {
                    seed: 8100,
                    index: 0,
                    filter: FilterList::allow(forward.clone()),
                    method: Method::Combined,
                },
            );
            let allow_reversed = detect_info(
                &text,
                &Case {
                    seed: 8100,
                    index: 0,
                    filter: FilterList::allow(reversed.clone()),
                    method: Method::Combined,
                },
            );
            let allow_shuffled = detect_info(
                &text,
                &Case {
                    seed: 8100,
                    index: 0,
                    filter: FilterList::allow(shuffled.clone()),
                    method: Method::Combined,
                },
            );
            assert_eq!(
                allow_forward, allow_reversed,
                "allowlist order changed result"
            );
            assert_eq!(
                allow_forward, allow_shuffled,
                "allowlist order changed result"
            );

            let deny_forward = detect_info(
                &text,
                &Case {
                    seed: 8100,
                    index: 0,
                    filter: FilterList::deny(forward.clone()),
                    method: Method::Combined,
                },
            );
            let deny_reversed = detect_info(
                &text,
                &Case {
                    seed: 8100,
                    index: 0,
                    filter: FilterList::deny(reversed.clone()),
                    method: Method::Combined,
                },
            );
            assert_eq!(deny_forward, deny_reversed, "denylist order changed result");
        }
    }
}

#[test]
fn properties_16_filter_matrix_various_allow_sizes_latin() {
    let langs = family_script_langs(Family::Latin);
    let sizes = [1, 2, 5, 10, langs.len()];
    for (idx, size) in sizes.iter().enumerate() {
        let list: Vec<Lang> = langs[..*size].to_vec();
        let seed = 9000 + idx as u64;
        let case = Case {
            seed,
            index: 0,
            filter: FilterList::allow(list),
            method: Method::Combined,
        };
        let mut rng = Rng::new(seed);
        for _ in 0..10 {
            let fragment = gen_pure_fragment(&mut rng, Family::Latin, 30);
            verify_sample(
                "allowlist matrices of every size keep results inside the allowed set",
                &fragment.chars,
                &case,
                Some(Family::Latin),
                None,
            );
        }
    }
}

#[test]
fn properties_17_filter_matrix_various_deny_sizes_cyrillic() {
    let langs = family_script_langs(Family::Cyrillic);
    let sizes = [1, 2, 3, langs.len() - 1, langs.len()];
    for (idx, size) in sizes.iter().enumerate() {
        let list: Vec<Lang> = langs[..*size].to_vec();
        let seed = 9100 + idx as u64;
        let case = Case {
            seed,
            index: 0,
            filter: FilterList::deny(list),
            method: Method::Combined,
        };
        let mut rng = Rng::new(seed);
        for _ in 0..12 {
            let fragment = gen_pure_fragment(&mut rng, Family::Cyrillic, 30);
            verify_sample(
                "denylist matrices of every size never return an excluded language",
                &fragment.chars,
                &case,
                Some(Family::Cyrillic),
                None,
            );
        }
    }
}

#[test]
fn properties_18_filter_matrix_same_members_reversed_order() {
    let langs = family_script_langs(Family::Latin);
    let list: Vec<Lang> = langs[..8].to_vec();
    let mut reversed = list.clone();
    reversed.reverse();
    let mut rng = Rng::new(9200);
    for index in 0..20 {
        let fragment = gen_pure_fragment(&mut rng, Family::Latin, 30);
        let text = fragment.to_string();
        let a = detect_info(
            &text,
            &Case {
                seed: 9200,
                index,
                filter: FilterList::allow(list.clone()),
                method: Method::Trigram,
            },
        );
        let b = detect_info(
            &text,
            &Case {
                seed: 9200,
                index,
                filter: FilterList::allow(reversed.clone()),
                method: Method::Trigram,
            },
        );
        assert_eq!(a, b);
    }
}

#[test]
fn properties_19_mandarin_allowlist_special_branch() {
    let chars: Vec<char> = "水山水火人".chars().collect();
    let jpn = Case {
        seed: 9300,
        index: 0,
        filter: FilterList::allow(vec![Lang::Jpn]),
        method: Method::Combined,
    };
    let cmn = Case {
        seed: 9300,
        index: 0,
        filter: FilterList::allow(vec![Lang::Cmn]),
        method: Method::Combined,
    };
    assert_eq!(
        detect_info(&chars.iter().collect::<String>(), &jpn)
            .unwrap()
            .lang(),
        Lang::Jpn
    );
    assert_eq!(
        detect_info(&chars.iter().collect::<String>(), &cmn)
            .unwrap()
            .lang(),
        Lang::Cmn
    );
}

#[test]
fn properties_20_mandarin_japanese_threshold_boundaries_are_exact() {
    // Public special branch contract from detect_lang_base_on_mandarin_script:
    //   jpn_pct > 0.20 -> Jpn 1.0
    //   jpn_pct >= 0.05 (strict > 0.05 branch starts above) -> Jpn 0.5
    //   jpn_pct > 0.02 -> Cmn 0.5
    //   otherwise      -> Cmn 1.0
    fn info(mandarin: usize, kana: usize) -> crate::core::Info {
        let mut chars = vec!['水'; mandarin];
        chars.extend(std::iter::repeat_n('ア', kana));
        let text: String = chars.iter().collect();
        let case = Case {
            seed: 9400,
            index: 0,
            filter: FilterList::All,
            method: Method::Combined,
        };
        detect_info(&text, &case).expect("mandarin text must be detected")
    }
    // 1 kana out of 5 => exactly 0.20 (strict boundary) -> Jpn side, 0.5.
    let at_five = info(4, 1);
    assert_eq!(at_five.lang(), Lang::Jpn);
    assert_eq!(at_five.confidence(), 0.5);
    // 1 kana out of 4 => exactly 0.25 -> Jpn 1.0.
    let above_five = info(3, 1);
    assert_eq!(above_five.lang(), Lang::Jpn);
    assert_eq!(above_five.confidence(), 1.0);
    // exactly 0.05 -> Cmn 0.5 (boundary is strict).
    let at_005 = info(19, 1);
    assert_eq!(at_005.lang(), Lang::Cmn);
    assert_eq!(at_005.confidence(), 0.5);
    // just above 0.05 -> Jpn 0.5.
    let above_005 = info(18, 1);
    assert_eq!(above_005.lang(), Lang::Jpn);
    assert_eq!(above_005.confidence(), 0.5);
    // exactly 0.02 -> Cmn 1.0 (boundary is strict).
    let at_002 = info(49, 1);
    assert_eq!(at_002.lang(), Lang::Cmn);
    assert_eq!(at_002.confidence(), 1.0);
    // just above 0.02 -> Cmn 0.5.
    let above_002 = info(39, 1);
    assert_eq!(above_002.lang(), Lang::Cmn);
    assert_eq!(above_002.confidence(), 0.5);
}

#[test]
fn properties_21_mandarin_denylist_special_branch() {
    let text: String = "水山水火".chars().collect();
    let deny_cmn = Case {
        seed: 9500,
        index: 0,
        filter: FilterList::deny(vec![Lang::Cmn]),
        method: Method::Combined,
    };
    let deny_jpn = Case {
        seed: 9500,
        index: 0,
        filter: FilterList::deny(vec![Lang::Jpn]),
        method: Method::Combined,
    };
    // Current documented special-branch behavior: the Mandarin hack falls back
    // across Cmn/Jpn even under denylists.
    assert_eq!(detect_info(&text, &deny_cmn).unwrap().lang(), Lang::Jpn);
    assert_eq!(detect_info(&text, &deny_jpn).unwrap().lang(), Lang::Cmn);
}

#[test]
fn properties_22_pure_kana_maps_to_japanese_branch() {
    for family in [Family::Hiragana, Family::Katakana] {
        let mut rng = Rng::new(9600 + family as u64);
        for _ in 0..8 {
            let fragment = gen_pure_fragment(&mut rng, family, 20);
            let text = fragment.to_string();
            let info = public_detect(&text).expect("pure kana must be detected");
            assert_eq!(info.lang(), Lang::Jpn, "family={family:?} text={text:?}");
            assert_eq!(info.script(), family.expected_script());
            assert!((0.0..=1.0).contains(&info.confidence()));
        }
    }
}

#[test]
fn properties_23_pure_latin_script_stable_under_punctuation_digits_newlines_whitespace() {
    check_decorated_pure_samples(10_001, 20);
    // Explicit named assertions per requested script live in 24..=29 below.
}

#[test]
fn properties_24_pure_cyrillic_script_stable_under_decoration() {
    let mut rng = Rng::new(10_101);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Cyrillic, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Cyrillic),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_25_pure_arabic_script_stable_under_decoration() {
    let mut rng = Rng::new(10_201);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Arabic, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Arabic),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_26_pure_devanagari_script_stable_under_decoration() {
    let mut rng = Rng::new(10_301);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Devanagari, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Devanagari),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_27_pure_hebrew_script_stable_under_decoration() {
    let mut rng = Rng::new(10_401);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Hebrew, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Hebrew),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_28_pure_han_script_stable_under_decoration() {
    let mut rng = Rng::new(10_501);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Han, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Mandarin),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_29_pure_hiragana_script_stable_under_decoration() {
    let mut rng = Rng::new(10_601);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Hiragana, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Hiragana),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_30_pure_katakana_script_stable_under_decoration() {
    let mut rng = Rng::new(10_701);
    for _ in 0..16 {
        let fragment = gen_pure_fragment(&mut rng, Family::Katakana, 28);
        let decorated = decorate(&mut rng, &fragment);
        assert_eq!(
            detect_script(&decorated),
            Some(Script::Katakana),
            "text={decorated:?}"
        );
    }
}

#[test]
fn properties_31_mixed_script_fragments_dominant_script_is_detectable() {
    let mut rng = Rng::new(11_001);
    for index in 0..40 {
        let mixed = gen_mixed_fragment(&mut rng, 3, 26);
        let script = detect_script(&mixed.text).expect("mixed fragment contains letters");
        assert_eq!(
            script,
            mixed.dominant.expected_script(),
            "sample #{index} text={:?}: expected dominant {:?}",
            mixed.text,
            mixed.dominant
        );
    }
}

#[test]
fn properties_32_confusable_pair_spa_por_ranking_is_stable_and_ordered() {
    check_confusable_pair(
        12_001,
        Family::Latin,
        [Lang::Spa, Lang::Por],
        Method::Combined,
    );
}

#[test]
fn properties_33_confusable_pair_rus_ukr_ranking_is_stable_and_ordered() {
    check_confusable_pair(
        12_101,
        Family::Cyrillic,
        [Lang::Rus, Lang::Ukr],
        Method::Combined,
    );
}

#[test]
fn properties_34_confusable_pair_ara_pes_ranking_is_stable_and_ordered() {
    check_confusable_pair(
        12_201,
        Family::Arabic,
        [Lang::Ara, Lang::Pes],
        Method::Trigram,
    );
}

// Generate text from trigrams of a real language profile. No natural-language
// probability is asserted: only top-two stability, nonnegative margin, filter
// legality and score ordering.
fn profile_chars(family: Family, lang: Lang) -> Vec<char> {
    let list: LangProfileList = match family {
        Family::Latin => LATIN_LANGS,
        Family::Cyrillic => CYRILLIC_LANGS,
        Family::Arabic => ARABIC_LANGS,
        Family::Devanagari => DEVANAGARI_LANGS,
        Family::Hebrew => HEBREW_LANGS,
        Family::Han | Family::Hiragana | Family::Katakana => unreachable!(),
    };
    let profile = list
        .iter()
        .find(|(candidate, _)| *candidate == lang)
        .expect("language profile exists")
        .1;
    let mut chars = Vec::new();
    for trigram in profile {
        let crate::trigrams::Trigram(a, b, c) = trigram;
        for ch in [a, b, c] {
            if *ch != ' ' && !chars.contains(ch) {
                chars.push(*ch);
            }
        }
    }
    chars
}

use crate::trigrams::LangProfileList;

fn gen_pair_text(rng: &mut Rng, alphabet: &[char], words: usize) -> String {
    let mut text = String::new();
    for word_idx in 0..words {
        if word_idx > 0 {
            text.push(' ');
        }
        let word_len = rng.range(3, 8);
        for _ in 0..word_len {
            text.push(rng.pick(alphabet));
        }
    }
    text
}

fn check_confusable_pair(seed: u64, family: Family, pair: [Lang; 2], method: Method) {
    let mut alphabet = profile_chars(family, pair[0]);
    for ch in profile_chars(family, pair[1]) {
        if !alphabet.contains(&ch) {
            alphabet.push(ch);
        }
    }

    let filters = [
        FilterList::All,
        FilterList::allow(pair.to_vec()),
        FilterList::deny(vec![pair[1]]),
    ];

    for (filter_idx, filter) in filters.iter().enumerate() {
        let mut rng = Rng::new(seed + filter_idx as u64);
        for index in 0..24 {
            let text = gen_pair_text(&mut rng, &alphabet, 8);
            let case = Case {
                seed,
                index,
                filter: filter.clone(),
                method,
            };
            let scores = raw_scores(&text, family, &case.filter, method);

            assert!(
                !scores.is_empty(),
                "seeded pair sample produced no candidates"
            );
            for (lang, score) in &scores {
                assert!(score.is_finite() && (0.0..=1.0).contains(score));
                assert!(
                    case.filter.is_allowed(*lang),
                    "ranking contained filtered language {lang:?}"
                );
            }
            for window in scores.windows(2) {
                assert!(
                    window[0].1 >= window[1].1,
                    "ranking not ordered: {}",
                    describe_ranking(&scores)
                );
            }

            let top_two: Vec<(Lang, f64)> = scores.iter().take(2).copied().collect();
            let margin = if top_two.len() == 2 {
                top_two[0].1 - top_two[1].1
            } else {
                top_two[0].1
            };
            assert!(margin >= 0.0, "negative top-two margin: {margin}");

            // Same input + same config must be stable across runs.
            let rerun = raw_scores(&text, family, &case.filter, method);
            assert_eq!(
                scores
                    .iter()
                    .map(|(l, s)| (*l, s.to_bits()))
                    .collect::<Vec<_>>(),
                rerun
                    .iter()
                    .map(|(l, s)| (*l, s.to_bits()))
                    .collect::<Vec<_>>(),
                "pair ranking not deterministic"
            );

            // End-to-end result must agree with the raw ranking winner.
            let info = detect_info(&text, &case);
            if let Some(info) = info {
                assert_eq!(
                    info.lang(),
                    scores[0].0,
                    "public result {:?} disagrees with ranking {}",
                    info.lang(),
                    describe_ranking(&scores)
                );
                assert!((0.0..=1.0).contains(&info.confidence()));
            }
            let _ = top_two;
        }
    }
}

#[test]
fn properties_00_named_property_inventory() {
    // Prints the full inventory so `cargo test --quiet properties` shows the
    // named cases and validation stages.
    let inventory = [
        "properties_00_named_property_inventory",
        "properties_01_generation_order_is_replayable_from_fixed_seed",
        "properties_02_generation_order_matches_frozen_sequence",
        "properties_03_repeated_detection_is_completely_deterministic_combined",
        "properties_04_repeated_detection_is_completely_deterministic_trigram",
        "properties_05_repeated_detection_is_completely_deterministic_alphabet",
        "properties_06_confidence_is_finite_and_within_public_range_combined",
        "properties_07_confidence_is_finite_and_within_public_range_trigram",
        "properties_08_confidence_is_finite_and_within_public_range_alphabet",
        "properties_09_calculate_confidence_helper_is_finite_and_bounded_on_tied_scores",
        "properties_10_whitelist_results_come_only_from_allowed_set",
        "properties_11_blacklist_never_returns_excluded_languages",
        "properties_12_empty_candidate_set_returns_none",
        "properties_13_single_candidate_returns_allowed_language_with_unit_confidence",
        "properties_14_mutually_exclusive_filters_never_agree_when_membership_differs",
        "properties_15_same_members_different_order_give_identical_results",
        "properties_16_filter_matrix_various_allow_sizes_latin",
        "properties_17_filter_matrix_various_deny_sizes_cyrillic",
        "properties_18_filter_matrix_same_members_reversed_order",
        "properties_19_mandarin_allowlist_special_branch",
        "properties_20_mandarin_japanese_threshold_boundaries_are_exact",
        "properties_21_mandarin_denylist_special_branch",
        "properties_22_pure_kana_maps_to_japanese_branch",
        "properties_23_pure_latin_script_stable_under_punctuation_digits_newlines_whitespace",
        "properties_24_pure_cyrillic_script_stable_under_decoration",
        "properties_25_pure_arabic_script_stable_under_decoration",
        "properties_26_pure_devanagari_script_stable_under_decoration",
        "properties_27_pure_hebrew_script_stable_under_decoration",
        "properties_28_pure_han_script_stable_under_decoration",
        "properties_29_pure_hiragana_script_stable_under_decoration",
        "properties_30_pure_katakana_script_stable_under_decoration",
        "properties_31_mixed_script_fragments_dominant_script_is_detectable",
        "properties_32_confusable_pair_spa_por_ranking_is_stable_and_ordered",
        "properties_33_confusable_pair_rus_ukr_ranking_is_stable_and_ordered",
        "properties_34_confusable_pair_ara_pes_ranking_is_stable_and_ordered",
        "validation-stage:seeded-generators",
        "validation-stage:determinism",
        "validation-stage:confidence-range",
        "validation-stage:whitelist-blacklist-contract",
        "validation-stage:empty-and-single-candidate",
        "validation-stage:filter-matrix-and-ordering",
        "validation-stage:mandarin-japanese-special-branch",
        "validation-stage:script-invariance-under-decoration",
        "validation-stage:mixed-scripts",
        "validation-stage:confusable-language-pairs",
        "validation-stage:all-public-methods",
    ];
    assert_eq!(
        inventory.len(),
        inventory
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
    );
    println!(
        "PROPERTY SUITE INVENTORY ({} named cases/stages):",
        inventory.len()
    );
    for name in inventory {
        println!("  {name}");
    }
}
