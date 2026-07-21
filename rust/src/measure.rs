use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "bench", about = "A program to measure the performance.")]
struct Args {
    #[clap(short = 'k', long)]
    keys_filename: String,

    #[clap(short = 'b', long, default_value = "false")]
    base64: bool,
}

fn main() {
    let args = Args::parse();

    println!("keys_filename: {}", &args.keys_filename);

    let (bin_keys, str_keys) = if args.base64 {
        let mut keys = load_base64_file(&args.keys_filename);
        keys.sort_unstable();
        (keys, None)
    } else {
        let mut str_keys = load_file(&args.keys_filename);
        str_keys.sort_unstable();
        let bin_keys = str_keys
            .iter()
            .map(|s| s.as_bytes().to_vec())
            .collect::<Vec<_>>();
        (bin_keys, Some(str_keys))
    };

    keys_stat(&bin_keys, str_keys.as_ref());

    if let Some(str_keys) = str_keys.as_ref() {
        println!("[crawdad/trie]");
        let trie = crawdad::Trie::from_keys(str_keys).unwrap();
        print_heap_bytes(trie.heap_bytes());
    }

    if let Some(str_keys) = str_keys.as_ref() {
        println!("[crawdad/mptrie]");
        let trie = crawdad::MpTrie::from_keys(str_keys).unwrap();
        print_heap_bytes(trie.heap_bytes());
    }

    if let Some(str_keys) = str_keys.as_ref() {
        println!("[yada]");
        let data = yada::builder::DoubleArrayBuilder::build(
            &str_keys
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, key)| (key, u32::try_from(i).unwrap()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        print_heap_bytes(data.len());
    }

    {
        println!("[fst/map]");
        let map = fst::raw::Fst::from_iter_map(
            bin_keys
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, key)| (key, i.try_into().unwrap())),
        )
        .unwrap();
        print_heap_bytes(map.as_bytes().len());
    }

    {
        println!("[daachorse/bytewise]");
        let pma = daachorse::DoubleArrayAhoCorasick::<u64>::new(&bin_keys).unwrap();
        print_heap_bytes(pma.heap_bytes());
    }

    if let Some(str_keys) = str_keys.as_ref() {
        println!("[daachorse/charwise]");
        let pma = daachorse::CharwiseDoubleArrayAhoCorasick::<u32>::new(str_keys).unwrap();
        print_heap_bytes(pma.heap_bytes());
    }

    {
        println!("[aho-corasick/nfa]");
        let pma = aho_corasick::AhoCorasick::new(&bin_keys).unwrap();
        print_heap_bytes(pma.memory_usage());
    }

    {
        println!("[aho-corasick/dfa]");
        let pma = aho_corasick::AhoCorasickBuilder::new()
            .kind(Some(aho_corasick::AhoCorasickKind::DFA))
            .build(&bin_keys)
            .unwrap();
        print_heap_bytes(pma.memory_usage());
    }
}

fn print_heap_bytes(bytes: usize) {
    println!(
        "heap_bytes: {} bytes, {:.3} MiB",
        bytes,
        bytes as f64 / (1024.0 * 1024.0)
    );
}

fn load_file<P>(path: P) -> Vec<String>
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

fn keys_stat(bin_keys: &[Vec<u8>], str_keys: Option<&Vec<String>>) {
    let totlen_byte = bin_keys.iter().fold(0, |acc, k| acc + k.len());
    let filesize = totlen_byte + bin_keys.len(); // with '\n'
    println!(
        "filesize: {} bytes, {:.3} MiB",
        filesize,
        filesize as f64 / (1024.0 * 1024.0)
    );
    println!("#keys: {}", bin_keys.len());
    println!(
        "#bytes/key: {:.3}",
        (totlen_byte as f64) / (bin_keys.len() as f64)
    );
    if let Some(str_keys) = str_keys {
        let totlen_char = str_keys.iter().fold(0, |acc, k| acc + k.chars().count());
        println!(
            "#chars/key: {:.3}",
            (totlen_char as f64) / (str_keys.len() as f64)
        );
    }
}
