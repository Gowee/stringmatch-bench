use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion, Throughput};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::time::Duration;

const SAMPLE: usize = 50;
const WARM: Duration = Duration::from_secs(2);
const MEASURE: Duration = Duration::from_secs(5);

fn load_lines<P: AsRef<Path>>(p: P) -> Vec<String> {
    BufReader::new(File::open(p).unwrap())
        .lines()
        .map(|l| l.unwrap())
        .collect()
}

fn load_whole<P: AsRef<Path>>(p: P) -> Vec<String> {
    let mut s = String::new();
    BufReader::new(File::open(p).unwrap())
        .read_to_string(&mut s)
        .unwrap();
    vec![s]
}

fn arms(
    group: &mut BenchmarkGroup<'_, criterion::measurement::WallTime>,
    pats: &[String],
    hs: &[String],
) {
    let total_bytes: u64 = hs.iter().map(|h| h.len()).sum::<usize>() as u64;
    group.throughput(Throughput::Bytes(total_bytes));

    let leg: daachorse::CharwiseDoubleArrayAhoCorasick<u32> =
        daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
            .match_kind(daachorse::MatchKind::LeftmostLongest)
            .build(pats)
            .unwrap();
    let exp: daachorse_exp::CharwiseDoubleArrayAhoCorasick<u32> =
        daachorse_exp::CharwiseDoubleArrayAhoCorasickBuilder::new()
            .match_kind(daachorse_exp::MatchKind::LeftmostLongestExperimental)
            .build(pats)
            .unwrap();

    group.bench_function("daachorse/charwise/u32", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for h in hs {
                for m in leg.leftmost_find_iter(h) {
                    sum += m.start() + m.end() + m.value() as usize;
                }
            }
            if sum == 0 {
                panic!("No matches found");
            }
        })
    });
    group.bench_function("daachorse-exp/owned-iter/u32", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for h in hs {
                for m in exp.leftmost_find_iter_experimental(h) {
                    sum += m.start() + m.end() + m.value() as usize;
                }
            }
            if sum == 0 {
                panic!("No matches found");
            }
        })
    });
    let mut finder = exp.leftmost_experimental_finder();
    group.bench_function("daachorse-exp/finder/u32", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for h in hs {
                for m in finder.find_iter(h) {
                    sum += m.start() + m.end() + m.value() as usize;
                }
            }
            if sum == 0 {
                panic!("No matches found");
            }
        })
    });
}

macro_rules! define {
    ($f:ident, $g:literal, $corpus:literal, $hay:literal, $whole:expr) => {
        fn $f(c: &mut Criterion) {
            let mut g = c.benchmark_group($g);
            g.sample_size(SAMPLE);
            g.warm_up_time(WARM);
            g.measurement_time(MEASURE);
            let mut pats = load_lines($corpus);
            pats.sort_unstable();
            let hs = if $whole {
                load_whole($hay)
            } else {
                load_lines($hay)
            };
            arms(&mut g, &pats, &hs);
            g.finish();
        }
    };
}

define!(
    w100_long,
    "words_100/longtext_leftmost_longest",
    "../data/words_100.txt",
    "../data/sherlock.txt",
    true
);
define!(
    w100_line,
    "words_100/find_leftmost_longest",
    "../data/words_100.txt",
    "../data/sherlock.txt",
    false
);

define!(
    w5k_long,
    "words_5000/longtext_leftmost_longest",
    "../data/words_5000.txt",
    "../data/sherlock.txt",
    true
);
define!(
    w5k_line,
    "words_5000/find_leftmost_longest",
    "../data/words_5000.txt",
    "../data/sherlock.txt",
    false
);

define!(
    w15k_long,
    "words_15000/longtext_leftmost_longest",
    "../data/words_15000.txt",
    "../data/sherlock.txt",
    true
);
define!(
    w15k_line,
    "words_15000/find_leftmost_longest",
    "../data/words_15000.txt",
    "../data/sherlock.txt",
    false
);

define!(
    w100k_long,
    "words_100000/longtext_leftmost_longest",
    "../data/words_100000.txt",
    "../data/sherlock.txt",
    true
);
define!(
    w100k_line,
    "words_100000/find_leftmost_longest",
    "../data/words_100000.txt",
    "../data/sherlock.txt",
    false
);

define!(
    unidic_long,
    "unidic/longtext_leftmost_longest",
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt",
    true
);
define!(
    unidic_line,
    "unidic/find_leftmost_longest",
    "../data/unidic/unidic",
    "../data/wagahaiwa_nekodearu.txt",
    false
);

criterion_group!(
    benches,
    w100_long,
    w100_line,
    w5k_long,
    w5k_line,
    w15k_long,
    w15k_line,
    w100k_long,
    w100k_line,
    unidic_long,
    unidic_line,
);
criterion_main!(benches);
