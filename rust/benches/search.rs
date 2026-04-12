use std::fs::File;
use std::hash::Hash;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use aho_corasick::AhoCorasickKind;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode,
};

const BUILD_SAMPLE_SIZE: usize = 10;
const BUILD_WARM_UP_TIME: Duration = Duration::from_millis(500);
const BUILD_MEASURE_TIME: Duration = Duration::from_secs(2);

const SEARCH_SAMPLE_SIZE: usize = 30;
const SEARCH_WARM_UP_TIME: Duration = Duration::from_millis(500);
const SEARCH_MEASURE_TIME: Duration = Duration::from_secs(2);

macro_rules! define_build_bench {
    ( $func_name:ident, $bench:ident, $group:literal, $loader:ident, $corpus:literal ) => {
        fn $func_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($group);
            group.sample_size(BUILD_SAMPLE_SIZE);
            group.warm_up_time(BUILD_WARM_UP_TIME);
            group.measurement_time(BUILD_MEASURE_TIME);
            group.sampling_mode(SamplingMode::Flat);
            let mut patterns = $loader($corpus);
            patterns.sort_unstable();
            $bench(&mut group, &patterns);
        }
    };
}

macro_rules! define_find_bench {
    ( $func_name:ident, $bench:ident, $group:literal, $loader:ident, $corpus:literal, $haystack:literal ) => {
        fn $func_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($group);
            group.sample_size(SEARCH_SAMPLE_SIZE);
            group.warm_up_time(SEARCH_WARM_UP_TIME);
            group.measurement_time(SEARCH_MEASURE_TIME);
            group.sampling_mode(SamplingMode::Flat);
            let mut patterns = $loader($corpus);
            patterns.sort_unstable();
            let haystacks = load_text_file($haystack);
            $bench(&mut group, &patterns, &haystacks);
        }
    };
}

macro_rules! daachorse_bench {
    ( $b:ident, $pma:ident, $func_name:ident, $haystacks:ident ) => {
        $b.iter(|| {
            let mut sum = 0;
            for haystack in $haystacks {
                for m in $pma.$func_name(haystack) {
                    sum += m.start() + m.end() + m.value() as usize;
                }
            }
            if sum == 0 {
                panic!();
            }
        });
    };
}

macro_rules! aho_corasick_bench {
    ( $b:ident, $pma:ident, $func_name:ident, $haystacks:ident ) => {
        $b.iter(|| {
            let mut sum = 0;
            for haystack in $haystacks {
                for m in $pma.$func_name(haystack) {
                    sum += m.start() + m.end() + m.pattern().as_usize();
                }
            }
            if sum == 0 {
                panic!();
            }
        });
    };
}

fn add_bytewise_build_benches<P>(group: &mut BenchmarkGroup<WallTime>, patterns: &[P])
where
    P: AsRef<[u8]> + Clone + Hash + Eq,
{
    group.bench_function("daachorse/bytewise/u64", |b| {
        b.iter(|| daachorse::DoubleArrayAhoCorasick::<u64>::new(patterns).unwrap());
    });
    group.bench_function("daachorse/bytewise/u32", |b| {
        b.iter(|| daachorse::DoubleArrayAhoCorasick::<u32>::new(patterns).unwrap());
    });
    group.bench_function("aho_corasick/nfa", |b| {
        b.iter(|| aho_corasick::AhoCorasick::new(patterns));
    });
    group.bench_function("aho_corasick/dfa", |b| {
        b.iter(|| {
            aho_corasick::AhoCorasickBuilder::new()
                .kind(Some(aho_corasick::AhoCorasickKind::DFA))
                .build(patterns)
                .unwrap()
        });
    });
    group.bench_function("yada", |b| {
        b.iter(|| {
            yada::builder::DoubleArrayBuilder::build(
                &patterns
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, pattern)| (pattern, i.try_into().unwrap()))
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        });
    });
    group.bench_function("fst/map", |b| {
        b.iter(|| {
            fst::raw::Fst::from_iter_map(
                patterns
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, pattern)| (pattern, i.try_into().unwrap())),
            )
            .unwrap()
        });
    });
    group.bench_function("hashbrown/HashMap", |b| {
        b.iter(|| {
            let mut map = hashbrown::HashMap::new();
            for (i, key) in patterns.iter().enumerate() {
                map.insert(key, i as u32);
            }
            map
        });
    });
}

fn add_charwise_build_benches(group: &mut BenchmarkGroup<WallTime>, patterns: &[String]) {
    group.bench_function("daachorse/charwise/u64", |b| {
        b.iter(|| daachorse::CharwiseDoubleArrayAhoCorasick::<u64>::new(patterns).unwrap());
    });
    group.bench_function("daachorse/charwise/u32", |b| {
        b.iter(|| daachorse::CharwiseDoubleArrayAhoCorasick::<u32>::new(patterns).unwrap());
    });
    group.bench_function("crawdad/trie", |b| {
        b.iter(|| crawdad::Trie::from_keys(patterns).unwrap());
    });
    group.bench_function("crawdad/mptrie", |b| {
        b.iter(|| crawdad::MpTrie::from_keys(patterns).unwrap());
    });
}

fn add_bytewise_find_benches<P>(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[P],
    haystacks: &[String],
) where
    P: AsRef<[u8]> + Clone + Hash + Eq,
{
    group.bench_function("daachorse/bytewise/u64", |b| {
        let pma = daachorse::DoubleArrayAhoCorasick::<u64>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_iter, haystacks);
    });
    group.bench_function("daachorse/bytewise/u32", |b| {
        let pma = daachorse::DoubleArrayAhoCorasick::<u32>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_iter, haystacks);
    });
    group.bench_function("aho_corasick/nfa", |b| {
        let pma = aho_corasick::AhoCorasick::new(patterns).unwrap();
        aho_corasick_bench!(b, pma, find_iter, haystacks);
    });
    group.bench_function("aho_corasick/dfa", |b| {
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .kind(Some(aho_corasick::AhoCorasickKind::DFA))
            .build(patterns)
            .unwrap();
        aho_corasick_bench!(b, pma, find_iter, haystacks);
    });
}

fn add_charwise_find_benches(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[String],
    haystacks: &[String],
) {
    group.bench_function("daachorse/charwise/u64", |b| {
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u64>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_iter, haystacks);
    });
    group.bench_function("daachorse/charwise/u32", |b| {
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u32>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_iter, haystacks);
    });
}

fn add_bytewise_find_overlapping_benches<P>(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[P],
    haystacks: &[String],
) where
    P: AsRef<[u8]> + Clone + Hash + Eq,
{
    group.bench_function("daachorse/bytewise/u64", |b| {
        let pma = daachorse::DoubleArrayAhoCorasick::<u64>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_iter, haystacks);
    });
    group.bench_function("daachorse/bytewise/u32", |b| {
        let pma = daachorse::DoubleArrayAhoCorasick::<u32>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_iter, haystacks);
    });
    group.bench_function("daachorse/bytewise/u64/no_suffix", |b| {
        let pma = daachorse::DoubleArrayAhoCorasick::<u64>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_no_suffix_iter, haystacks);
    });
    group.bench_function("daachorse/bytewise/u32/no_suffix", |b| {
        let pma = daachorse::DoubleArrayAhoCorasick::<u32>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_no_suffix_iter, haystacks);
    });
    group.bench_function("aho_corasick/nfa", |b| {
        let pma = aho_corasick::AhoCorasick::new(patterns).unwrap();
        aho_corasick_bench!(b, pma, find_overlapping_iter, haystacks);
    });
    group.bench_function("aho_corasick/dfa", |b| {
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .kind(Some(aho_corasick::AhoCorasickKind::DFA))
            .build(patterns)
            .unwrap();
        aho_corasick_bench!(b, pma, find_overlapping_iter, haystacks);
    });
    group.bench_function("yada", |b| {
        let data = yada::builder::DoubleArrayBuilder::build(
            &patterns
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, pattern)| (pattern, i as u32))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let da = yada::DoubleArray::new(data);
        b.iter(|| {
            let mut sum = 0;
            for haystack in haystacks {
                let haystack_bytes = haystack.as_bytes();
                for i in 0..haystack_bytes.len() {
                    for (id, length) in da.common_prefix_search(&haystack_bytes[i..]) {
                        sum += i + length + id as usize;
                    }
                }
            }
            if sum == 0 {
                panic!();
            }
        });
    });
    group.bench_function("fst/map", |b| {
        let fst = fst::raw::Fst::from_iter_map(
            patterns
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, pattern)| (pattern, i as u64)),
        )
        .unwrap();
        b.iter(|| {
            let mut sum = 0;
            for haystack in haystacks {
                let haystack_bytes = haystack.as_bytes();
                for i in 0..haystack_bytes.len() {
                    for (id, length) in fst_common_prefix_search(&fst, &haystack_bytes[i..]) {
                        sum += i + length as usize + id as usize;
                    }
                }
            }
            if sum == 0 {
                panic!();
            }
        });
    });
}

fn add_charwise_find_overlapping_benches(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[String],
    haystacks: &[String],
) {
    group.bench_function("daachorse/charwise/u64", |b| {
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u64>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_iter, haystacks);
    });
    group.bench_function("daachorse/charwise/u32", |b| {
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u32>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_iter, haystacks);
    });
    group.bench_function("daachorse/charwise/u64/no_suffix", |b| {
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u64>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_no_suffix_iter, haystacks);
    });
    group.bench_function("daachorse/charwise/u32/no_suffix", |b| {
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u32>::new(patterns).unwrap();
        daachorse_bench!(b, pma, find_overlapping_no_suffix_iter, haystacks);
    });
    group.bench_function("crawdad/trie", |b| {
        let da = crawdad::Trie::from_keys(patterns).unwrap();
        b.iter(|| {
            let mut sum = 0;
            let mut haystack_chars = vec![];
            for haystack in haystacks {
                haystack_chars.clear();
                haystack_chars.extend(haystack.chars());
                for i in 0..haystack_chars.len() {
                    for (id, length) in da.common_prefix_search(haystack_chars[i..].iter().copied())
                    {
                        sum += i + length + id as usize;
                    }
                }
            }
            if sum == 0 {
                panic!();
            }
        });
    });
    group.bench_function("crawdad/mptrie", |b| {
        let da = crawdad::MpTrie::from_keys(patterns).unwrap();
        b.iter(|| {
            let mut sum = 0;
            let mut haystack_chars = vec![];
            for haystack in haystacks {
                haystack_chars.clear();
                haystack_chars.extend(haystack.chars());
                for i in 0..haystack_chars.len() {
                    for (id, length) in da.common_prefix_search(haystack_chars[i..].iter().copied())
                    {
                        sum += i + length + id as usize;
                    }
                }
            }
            if sum == 0 {
                panic!();
            }
        });
    });
}

fn add_bytewise_find_leftmost_longest_benches<P>(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[P],
    haystacks: &[String],
) where
    P: AsRef<[u8]> + Clone + Hash + Eq,
{
    group.bench_function("daachorse/bytewise/u64", |b| {
        let pma: daachorse::DoubleArrayAhoCorasick<u64> =
            daachorse::DoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostLongest)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
    group.bench_function("daachorse/bytewise/u32", |b| {
        let pma: daachorse::DoubleArrayAhoCorasick<u32> =
            daachorse::DoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostLongest)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
    group.bench_function("aho_corasick/nfa", |b| {
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::LeftmostLongest)
            .build(patterns)
            .unwrap();
        aho_corasick_bench!(b, pma, find_iter, haystacks);
    });
    group.bench_function("aho_corasick/dfa", |b| {
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .kind(Some(aho_corasick::AhoCorasickKind::DFA))
            .match_kind(aho_corasick::MatchKind::LeftmostLongest)
            .build(patterns)
            .unwrap();
        aho_corasick_bench!(b, pma, find_iter, haystacks);
    });
}

fn add_charwise_find_leftmost_longest_benches(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[String],
    haystacks: &[String],
) {
    group.bench_function("daachorse/charwise/u64", |b| {
        let pma: daachorse::CharwiseDoubleArrayAhoCorasick<u64> =
            daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostLongest)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
    group.bench_function("daachorse/charwise/u32", |b| {
        let pma: daachorse::CharwiseDoubleArrayAhoCorasick<u32> =
            daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostLongest)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
}

fn add_bytewise_find_leftmost_first_benches<P>(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[P],
    haystacks: &[String],
) where
    P: AsRef<[u8]> + Clone + Hash + Eq,
{
    group.bench_function("daachorse/bytewise/u64", |b| {
        let pma: daachorse::DoubleArrayAhoCorasick<u64> =
            daachorse::DoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostFirst)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
    group.bench_function("daachorse/bytewise/u32", |b| {
        let pma: daachorse::DoubleArrayAhoCorasick<u32> =
            daachorse::DoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostFirst)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
    group.bench_function("aho_corasick/nfa", |b| {
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .build(patterns)
            .unwrap();
        aho_corasick_bench!(b, pma, find_iter, haystacks);
    });
    group.bench_function("aho_corasick/dfa", |b| {
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .kind(Some(AhoCorasickKind::DFA))
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .build(patterns)
            .unwrap();
        aho_corasick_bench!(b, pma, find_iter, haystacks);
    });
}

fn add_charwise_find_leftmost_first_benches(
    group: &mut BenchmarkGroup<WallTime>,
    patterns: &[String],
    haystacks: &[String],
) {
    group.bench_function("daachorse/charwise/u64", |b| {
        let pma: daachorse::CharwiseDoubleArrayAhoCorasick<u64> =
            daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostFirst)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
    group.bench_function("daachorse/charwise/u32", |b| {
        let pma: daachorse::CharwiseDoubleArrayAhoCorasick<u32> =
            daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostFirst)
                .build(patterns)
                .unwrap();
        daachorse_bench!(b, pma, leftmost_find_iter, haystacks);
    });
}

fn load_text_file<P>(path: P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let file = File::open(path).unwrap();
    let buf = BufReader::new(file);
    buf.lines().map(|line| line.unwrap()).collect()
}

fn load_base64_file<P>(path: P) -> Vec<Vec<u8>>
where
    P: AsRef<Path>,
{
    let file = File::open(path).unwrap();
    let buf = BufReader::new(file);
    buf.lines()
        .map(|line| BASE64_STANDARD.decode(line.unwrap()).unwrap())
        .collect()
}

fn fst_common_prefix_search<'a>(
    fst: &'a fst::raw::Fst<Vec<u8>>,
    s: &'a [u8],
) -> impl Iterator<Item = (u64, u64)> + 'a {
    s.iter()
        .scan(
            (0, fst.root(), fst::raw::Output::zero()),
            move |(pattern_len, node, output), &byte| {
                if let Some(b_index) = node.find_input(byte) {
                    let transition = node.transition(b_index);
                    *pattern_len += 1;
                    *output = output.cat(transition.out);
                    *node = fst.node(transition.addr);
                    return Some((node.is_final(), *pattern_len, output.value()));
                }
                None
            },
        )
        .filter_map(|(is_final, pattern_len, pattern_id)| {
            if is_final {
                Some((pattern_id, pattern_len))
            } else {
                None
            }
        })
}

// find
define_find_bench!(
    criterion_words_100_find_bytewise,
    add_bytewise_find_benches,
    "words_100/find",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100_find_charwise,
    add_charwise_find_benches,
    "words_100/find",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_bytewise,
    add_bytewise_find_benches,
    "words_5000/find",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_charwise,
    add_charwise_find_benches,
    "words_5000/find",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_bytewise,
    add_bytewise_find_benches,
    "words_15000/find",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_charwise,
    add_charwise_find_benches,
    "words_15000/find",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_bytewise,
    add_bytewise_find_benches,
    "words_100000/find",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_charwise,
    add_charwise_find_benches,
    "words_100000/find",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_cl100k_find,
    add_bytewise_find_benches,
    "cl100k/find",
    load_base64_file,
    "../data/tiktoken/cl100k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_o200k_find,
    add_bytewise_find_benches,
    "o200k/find",
    load_base64_file,
    "../data/tiktoken/o200k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_bytewise,
    add_bytewise_find_benches,
    "unidic/find",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_charwise,
    add_bytewise_find_benches,
    "unidic/find",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);

// find_overlapping
define_find_bench!(
    criterion_words_100_find_overlapping_bytewise,
    add_bytewise_find_overlapping_benches,
    "words_100/find_overlapping",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100_find_overlapping_charwise,
    add_charwise_find_overlapping_benches,
    "words_100/find_overlapping",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_overlapping_bytewise,
    add_bytewise_find_overlapping_benches,
    "words_5000/find_overlapping",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_overlapping_charwise,
    add_charwise_find_overlapping_benches,
    "words_5000/find_overlapping",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_overlapping_bytewise,
    add_bytewise_find_overlapping_benches,
    "words_15000/find_overlapping",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_overlapping_charwise,
    add_charwise_find_overlapping_benches,
    "words_15000/find_overlapping",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_overlapping_bytewise,
    add_bytewise_find_overlapping_benches,
    "words_100000/find_overlapping",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_overlapping_charwise,
    add_charwise_find_overlapping_benches,
    "words_100000/find_overlapping",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_cl100k_find_overlapping,
    add_bytewise_find_overlapping_benches,
    "cl100k/find_overlapping",
    load_base64_file,
    "../data/tiktoken/cl100k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_o200k_find_overlapping,
    add_bytewise_find_overlapping_benches,
    "o200k/find_overlapping",
    load_base64_file,
    "../data/tiktoken/o200k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_overlapping_bytewise,
    add_bytewise_find_overlapping_benches,
    "unidic/find_overlapping",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_overlapping_charwise,
    add_bytewise_find_overlapping_benches,
    "unidic/find_overlapping",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);

// find_leftmost_longest
define_find_bench!(
    criterion_words_100_find_leftmost_longest_bytewise,
    add_bytewise_find_leftmost_longest_benches,
    "words_100/find_leftmost_longest",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100_find_leftmost_longest_charwise,
    add_charwise_find_leftmost_longest_benches,
    "words_100/find_leftmost_longest",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_leftmost_longest_bytewise,
    add_bytewise_find_leftmost_longest_benches,
    "words_5000/find_leftmost_longest",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_leftmost_longest_charwise,
    add_charwise_find_leftmost_longest_benches,
    "words_5000/find_leftmost_longest",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_leftmost_longest_bytewise,
    add_bytewise_find_leftmost_longest_benches,
    "words_15000/find_leftmost_longest",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_leftmost_longest_charwise,
    add_charwise_find_leftmost_longest_benches,
    "words_15000/find_leftmost_longest",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_leftmost_longest_bytewise,
    add_bytewise_find_leftmost_longest_benches,
    "words_100000/find_leftmost_longest",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_leftmost_longest_charwise,
    add_charwise_find_leftmost_longest_benches,
    "words_100000/find_leftmost_longest",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_cl100k_find_leftmost_longest,
    add_bytewise_find_leftmost_longest_benches,
    "cl100k/find_leftmost_longest",
    load_base64_file,
    "../data/tiktoken/cl100k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_o200k_find_leftmost_longest,
    add_bytewise_find_leftmost_longest_benches,
    "o200k/find_leftmost_longest",
    load_base64_file,
    "../data/tiktoken/o200k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_leftmost_longest_bytewise,
    add_bytewise_find_leftmost_longest_benches,
    "unidic/find_leftmost_longest",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_leftmost_longest_charwise,
    add_bytewise_find_leftmost_longest_benches,
    "unidic/find_leftmost_longest",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);

// find_leftmost_first
define_find_bench!(
    criterion_words_100_find_leftmost_first_bytewise,
    add_bytewise_find_leftmost_first_benches,
    "words_100/find_leftmost_first",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100_find_leftmost_first_charwise,
    add_charwise_find_leftmost_first_benches,
    "words_100/find_leftmost_first",
    load_text_file,
    "../data/words_100.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_leftmost_first_bytewise,
    add_bytewise_find_leftmost_first_benches,
    "words_5000/find_leftmost_first",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_5000_find_leftmost_first_charwise,
    add_charwise_find_leftmost_first_benches,
    "words_5000/find_leftmost_first",
    load_text_file,
    "../data/words_5000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_leftmost_first_bytewise,
    add_bytewise_find_leftmost_first_benches,
    "words_15000/find_leftmost_first",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_15000_find_leftmost_first_charwise,
    add_charwise_find_leftmost_first_benches,
    "words_15000/find_leftmost_first",
    load_text_file,
    "../data/words_15000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_leftmost_first_bytewise,
    add_bytewise_find_leftmost_first_benches,
    "words_100000/find_leftmost_first",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_words_100000_find_leftmost_first_charwise,
    add_charwise_find_leftmost_first_benches,
    "words_100000/find_leftmost_first",
    load_text_file,
    "../data/words_100000.txt",
    "../data/sherlock.txt"
);
define_find_bench!(
    criterion_cl100k_find_leftmost_first,
    add_bytewise_find_leftmost_first_benches,
    "cl100k/find_leftmost_first",
    load_base64_file,
    "../data/tiktoken/cl100k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_o200k_find_leftmost_first,
    add_bytewise_find_leftmost_first_benches,
    "o200k/find_leftmost_first",
    load_base64_file,
    "../data/tiktoken/o200k_base.base64",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_leftmost_first_bytewise,
    add_bytewise_find_leftmost_first_benches,
    "unidic/find_leftmost_first",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);
define_find_bench!(
    criterion_unidic_find_leftmost_first_charwise,
    add_bytewise_find_leftmost_first_benches,
    "unidic/find_leftmost_first",
    load_text_file,
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt"
);

// build
define_build_bench!(
    criterion_words_100_build_bytewise,
    add_bytewise_build_benches,
    "words_100/build",
    load_text_file,
    "../data/words_100.txt"
);
define_build_bench!(
    criterion_words_100_build_charwise,
    add_charwise_build_benches,
    "words_100/build",
    load_text_file,
    "../data/words_100.txt"
);
define_build_bench!(
    criterion_words_5000_build_bytewise,
    add_bytewise_build_benches,
    "words_5000/build",
    load_text_file,
    "../data/words_5000.txt"
);
define_build_bench!(
    criterion_words_5000_build_charwise,
    add_charwise_build_benches,
    "words_5000/build",
    load_text_file,
    "../data/words_5000.txt"
);
define_build_bench!(
    criterion_words_15000_build_bytewise,
    add_bytewise_build_benches,
    "words_15000/build",
    load_text_file,
    "../data/words_15000.txt"
);
define_build_bench!(
    criterion_words_15000_build_charwise,
    add_charwise_build_benches,
    "words_15000/build",
    load_text_file,
    "../data/words_15000.txt"
);
define_build_bench!(
    criterion_words_100000_build_bytewise,
    add_bytewise_build_benches,
    "words_100000/build",
    load_text_file,
    "../data/words_100000.txt"
);
define_build_bench!(
    criterion_words_100000_build_charwise,
    add_charwise_build_benches,
    "words_100000/build",
    load_text_file,
    "../data/words_100000.txt"
);
define_build_bench!(
    criterion_cl100k_build,
    add_bytewise_build_benches,
    "cl100k/build",
    load_base64_file,
    "../data/toktoken/cl100k_base.base64"
);
define_build_bench!(
    criterion_o200k_build,
    add_bytewise_build_benches,
    "o200k/build",
    load_base64_file,
    "../data/toktoken/cl100k_base.base64"
);
define_build_bench!(
    criterion_unidic_build_bytewise,
    add_bytewise_build_benches,
    "unidic/build",
    load_text_file,
    "../data/unidic/unidic"
);
define_build_bench!(
    criterion_unidic_build_charwise,
    add_charwise_build_benches,
    "unidic/build",
    load_text_file,
    "../data/unidic/unidic"
);

criterion_group!(
    benches,
    criterion_words_100_find_bytewise,
    criterion_words_100_find_charwise,
    criterion_words_5000_find_bytewise,
    criterion_words_5000_find_charwise,
    criterion_words_15000_find_bytewise,
    criterion_words_15000_find_charwise,
    criterion_words_100000_find_bytewise,
    criterion_words_100000_find_charwise,
    criterion_cl100k_find,
    criterion_o200k_find,
    criterion_unidic_find_bytewise,
    criterion_unidic_find_charwise,
    criterion_words_100_find_overlapping_bytewise,
    criterion_words_100_find_overlapping_charwise,
    criterion_words_5000_find_overlapping_bytewise,
    criterion_words_5000_find_overlapping_charwise,
    criterion_words_15000_find_overlapping_bytewise,
    criterion_words_15000_find_overlapping_charwise,
    criterion_words_100000_find_overlapping_bytewise,
    criterion_words_100000_find_overlapping_charwise,
    criterion_cl100k_find_overlapping,
    criterion_o200k_find_overlapping,
    criterion_unidic_find_overlapping_bytewise,
    criterion_unidic_find_overlapping_charwise,
    criterion_words_100_find_leftmost_longest_bytewise,
    criterion_words_100_find_leftmost_longest_charwise,
    criterion_words_5000_find_leftmost_longest_bytewise,
    criterion_words_5000_find_leftmost_longest_charwise,
    criterion_words_15000_find_leftmost_longest_bytewise,
    criterion_words_15000_find_leftmost_longest_charwise,
    criterion_words_100000_find_leftmost_longest_bytewise,
    criterion_words_100000_find_leftmost_longest_charwise,
    criterion_cl100k_find_leftmost_longest,
    criterion_o200k_find_leftmost_longest,
    criterion_unidic_find_leftmost_longest_bytewise,
    criterion_unidic_find_leftmost_longest_charwise,
    criterion_words_100_find_leftmost_first_bytewise,
    criterion_words_100_find_leftmost_first_charwise,
    criterion_words_5000_find_leftmost_first_bytewise,
    criterion_words_5000_find_leftmost_first_charwise,
    criterion_words_15000_find_leftmost_first_bytewise,
    criterion_words_15000_find_leftmost_first_charwise,
    criterion_words_100000_find_leftmost_first_bytewise,
    criterion_words_100000_find_leftmost_first_charwise,
    criterion_cl100k_find_leftmost_first,
    criterion_o200k_find_leftmost_first,
    criterion_unidic_find_leftmost_first_bytewise,
    criterion_unidic_find_leftmost_first_charwise,
    criterion_words_100_build_bytewise,
    criterion_words_100_build_charwise,
    criterion_words_5000_build_bytewise,
    criterion_words_5000_build_charwise,
    criterion_words_15000_build_bytewise,
    criterion_words_15000_build_charwise,
    criterion_words_100000_build_bytewise,
    criterion_words_100000_build_charwise,
    criterion_cl100k_build,
    criterion_o200k_build,
    criterion_unidic_build_bytewise,
    criterion_unidic_build_charwise,
);
criterion_main!(benches);
