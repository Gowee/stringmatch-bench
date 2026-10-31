use std::fs::File;
use std::hint::black_box;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::time::Instant;


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

enum Mode {
    Legacy,
    Owned,
    Finder,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: profile_compare <legacy|owned|finder> <dataset> [reps] [--lines]");
        std::process::exit(1);
    }

    let mode_str = args[1].as_str();
    let mode = match mode_str {
        "legacy" | "leg" => Mode::Legacy,
        "owned" | "iter" | "exp_owned" => Mode::Owned,
        "finder" | "exp_finder" | "experimental" | "exp" => Mode::Finder,
        _ => panic!("Unknown mode: {}. Use legacy, owned, or finder.", mode_str),
    };
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
        "Building automaton for {} (mode: {}, patterns: {}, lines: {})...",
        dataset,
        mode_str,
        pats.len(),
        if per_line { "per-line" } else { "long-text" }
    );

    let (legacy_pma, linear_pma) = match mode {
        Mode::Legacy => {
            let pma: daachorse::CharwiseDoubleArrayAhoCorasick<u32> =
                daachorse::CharwiseDoubleArrayAhoCorasickBuilder::new()
                    .match_kind(daachorse::MatchKind::LeftmostLongest)
                    .build(&pats)
                    .unwrap();
            (Some(pma), None)
        }
        Mode::Owned | Mode::Finder => {
            let pma: daachorse_exp::CharwiseDoubleArrayAhoCorasick<u32> =
                daachorse_exp::CharwiseDoubleArrayAhoCorasickBuilder::new()
                    .match_kind(daachorse_exp::MatchKind::LeftmostLongestExperimental)
                    .build(&pats)
                    .unwrap();
            (None, Some(pma))
        }
    };

    eprintln!("Done building. Running {} reps...", reps);

    let mut linear_finder = match (&mode, &linear_pma) {
        (Mode::Finder, Some(pma)) => Some(pma.leftmost_experimental_finder()),
        _ => None,
    };

    // Warm up
    for _ in 0..3 {
        let mut sum = 0usize;
        for hay in &haystacks {
            match mode {
                Mode::Legacy => {
                    for m in legacy_pma.as_ref().unwrap().leftmost_find_iter(hay) {
                        sum += m.start() + m.end() + m.value() as usize;
                    }
                }
                Mode::Owned => {
                    for m in linear_pma.as_ref().unwrap().leftmost_find_iter_experimental(hay) {
                        sum += m.start() + m.end() + m.value() as usize;
                    }
                }
                Mode::Finder => {
                    for m in linear_finder.as_mut().unwrap().find_iter(hay) {
                        sum += m.start() + m.end() + m.value() as usize;
                    }
                }
            }
        }
        black_box(sum);
    }

    let mut perf_fifo = std::env::var("PERF_CTL").ok().map(|p| {
        std::fs::OpenOptions::new()
            .write(true)
            .open(p)
            .expect("Failed to open PERF_CTL fifo")
    });

    if let Some(f) = perf_fifo.as_mut() {
        use std::io::Write;
        f.write_all(b"enable\n").unwrap();
        f.flush().unwrap();
    }

    let start = Instant::now();
    let mut total_matches = 0;
    let mut total_checksum = 0u64;

    for _ in 0..reps {
        for hay in &haystacks {
            match mode {
                Mode::Legacy => {
                    for m in legacy_pma.as_ref().unwrap().leftmost_find_iter(hay) {
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
                Mode::Owned => {
                    for m in linear_pma.as_ref().unwrap().leftmost_find_iter_experimental(hay) {
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
                Mode::Finder => {
                    for m in linear_finder.as_mut().unwrap().find_iter(hay) {
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
            }
        }
    }

    if let Some(f) = perf_fifo.as_mut() {
        use std::io::Write;
        f.write_all(b"disable\n").unwrap();
        f.flush().unwrap();
    }

    let elapsed = start.elapsed();
    let per_rep = elapsed.as_secs_f64() / (reps as f64);
    let mib = total_bytes as f64 / (1024.0 * 1024.0);
    let thrpt = mib / per_rep;

    println!(
        "[{}] {} ({}): {:.3} ms/rep ({:.1} MiB/s), matches/rep: {}, checksum: {:#016x}",
        mode_str,
        dataset,
        if per_line { "per-line" } else { "long-text" },
        per_rep * 1e3,
        thrpt,
        total_matches / reps,
        total_checksum
    );
}
