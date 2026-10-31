use std::fs::File;
use std::hint::black_box;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::time::Instant;

enum Automaton {
    Legacy(daachorse::CharwiseDoubleArrayAhoCorasick<u32>),
    Linear(daachorse_exp::CharwiseDoubleArrayAhoCorasick<u32>),
}

fn load_lines<P: AsRef<Path>>(p: P) -> Vec<String> {
    BufReader::new(File::open(p).unwrap())
        .lines()
        .map(|l| l.unwrap())
        .collect()
}

fn load_whole<P: AsRef<Path>>(p: P) -> String {
    let mut s = String::new();
    BufReader::new(File::open(p).unwrap())
        .read_to_string(&mut s)
        .unwrap();
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: profile_compare <legacy|experimental> <dataset> [reps] [--lines]");
        std::process::exit(1);
    }

    let mode = &args[1];
    let dataset = &args[2];
    let reps: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    let per_line = args.iter().any(|a| a == "--lines");

    let (pats_path, hay_path) = match dataset.as_str() {
        "words_100" => ("/home/lotus/Workspace/stringmatch-bench/data/words_100.txt", "/home/lotus/Workspace/stringmatch-bench/data/sherlock.txt"),
        "words_5000" => ("/home/lotus/Workspace/stringmatch-bench/data/words_5000.txt", "/home/lotus/Workspace/stringmatch-bench/data/sherlock.txt"),
        "words_15000" => ("/home/lotus/Workspace/stringmatch-bench/data/words_15000.txt", "/home/lotus/Workspace/stringmatch-bench/data/sherlock.txt"),
        "words_100000" => ("/home/lotus/Workspace/stringmatch-bench/data/words_100000.txt", "/home/lotus/Workspace/stringmatch-bench/data/sherlock.txt"),
        "unidic" => ("/home/lotus/Workspace/stringmatch-bench/data/unidic/unidic", "/home/lotus/Workspace/stringmatch-bench/data/wagahaiwa_nekodearu.txt"),
        _ => panic!("Unknown dataset: {}", dataset),
    };

    let pats = load_lines(pats_path);
    let haystacks: Vec<String> = if per_line {
        load_lines(hay_path)
    } else {
        vec![load_whole(hay_path)]
    };
    let total_bytes: usize = haystacks.iter().map(|h| h.len()).sum();

    eprintln!(
        "Building {} automaton for {} (patterns: {}, mode: {})...",
        mode,
        dataset,
        pats.len(),
        if per_line { "per-line" } else { "long-text" }
    );

    let automaton = match mode.as_str() {
        "legacy" => {
            let pma = daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse::MatchKind::LeftmostLongest)
                .build(&pats)
                .unwrap();
            Automaton::Legacy(pma)
        }
        "base" | "eager" | "experimental" | "single" | "full" | "linear" | "exp" => {
            let pma = daachorse_exp::CharwiseDoubleArrayAhoCorasickBuilder::new()
                .match_kind(daachorse_exp::MatchKind::LeftmostLongestExperimental)
                .build(&pats)
                .unwrap();
            Automaton::Linear(pma)
        }
        _ => panic!("Unknown mode: {}", mode),
    };

    eprintln!("Done building. Running {} reps...", reps);

    let mut linear_finder = match &automaton {
        Automaton::Linear(pma) => Some(pma.leftmost_experimental_finder()),
        _ => None,
    };

    // Warm up
    for _ in 0..3 {
        let mut sum = 0usize;
        for hay in &haystacks {
            match (&automaton, &mut linear_finder) {
                (Automaton::Legacy(pma), _) => {
                    for m in pma.leftmost_find_iter(hay) {
                        sum += m.start() + m.end() + m.value() as usize;
                    }
                }
                (Automaton::Linear(_), Some(finder)) => {
                    for m in finder.find_iter(hay) {
                        sum += m.start() + m.end() + m.value() as usize;
                    }
                }
                _ => unreachable!(),
            }
        }
        black_box(sum);
    }

    let start = Instant::now();
    let mut total_matches = 0;
    let mut total_checksum = 0u64;

    for _ in 0..reps {
        for hay in &haystacks {
            match (&automaton, &mut linear_finder) {
                (Automaton::Legacy(pma), _) => {
                    for m in pma.leftmost_find_iter(hay) {
                        total_matches += 1;
                        total_checksum = total_checksum
                            .wrapping_mul(31)
                            .wrapping_add(m.start() as u64)
                            .wrapping_mul(31)
                            .wrapping_add(m.end() as u64)
                            .wrapping_mul(31)
                            .wrapping_add(m.value() as u64);
                    }
                }
                (Automaton::Linear(_), Some(finder)) => {
                    for m in finder.find_iter(hay) {
                        total_matches += 1;
                        total_checksum = total_checksum
                            .wrapping_mul(31)
                            .wrapping_add(m.start() as u64)
                            .wrapping_mul(31)
                            .wrapping_add(m.end() as u64)
                            .wrapping_mul(31)
                            .wrapping_add(m.value() as u64);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    let elapsed = start.elapsed();
    let per_rep = elapsed.as_secs_f64() / (reps as f64);
    let mib = total_bytes as f64 / (1024.0 * 1024.0);
    let thrpt = mib / per_rep;

    println!(
        "[{}] {} ({}): {:.3} ms/rep ({:.1} MiB/s), matches/rep: {}, checksum: {:#016x}",
        mode,
        dataset,
        if per_line { "per-line" } else { "long-text" },
        per_rep * 1e3,
        thrpt,
        total_matches / reps,
        total_checksum
    );
}
