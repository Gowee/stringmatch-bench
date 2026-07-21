# stringmatch-bench

## Tools

This repository provides benchmark tools to compare the performance of data structures for string matching.

### Rust

Directory `rust` provides benchmarks for the following libraries:

- [`aho_corasick::AhoCorasick`](https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasick.html)
- [`daachorse::DoubleArrayAhoCorasick`](https://docs.rs/daachorse/latest/daachorse/bytewise/struct.DoubleArrayAhoCorasick.html)
- [`daachorse::CharwiseDoubleArrayAhoCorasick`](https://docs.rs/daachorse/latest/daachorse/charwise/struct.CharwiseDoubleArrayAhoCorasick.html)
- [`crawdad::Trie`](https://docs.rs/crawdad/latest/crawdad/trie/struct.Trie.html)
- [`crawdad::MpTrie`](https://docs.rs/crawdad/latest/crawdad/mptrie/struct.MpTrie.html)
- [`yada::DoubleArray`](https://docs.rs/yada/latest/yada/struct.DoubleArray.html)
- [`fst::Map`](https://docs.rs/fst/latest/fst/struct.Map.html)
- [`hashbrown::HashMap`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashMap.html)

After moving your current directory to `rust`,
you can measure the memory usage of data structures built from your dataset with the following command:

```
$ cargo run --release --bin measure -- -k ../data/unidic/unidic
```

For base64-encoded datasets such as the tiktoken vocabularies, add the `-b` option:

```
$ cargo run --release --bin measure -- -k ../data/tiktoken/o200k_base.base64 -b
```

You can also measure build and search times with [`criterion.rs`](https://github.com/bheisler/criterion.rs):

```
$ cargo bench
```

### Python

Directory `python` provides benchmarks for the following libraries:

- [`ahocorasick_rs`](https://github.com/G-Research/ahocorasick_rs)
- [`daachorse`](https://pypi.org/project/daachorse/)

After moving your current directory to `python`,
you can measure search times with [`pytest-benchmark`](https://pytest-benchmark.readthedocs.io/) by running [`nox`](https://nox.thea.codes/):

```
$ nox
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

The datasets contained in [data](./data) are copied from third-party repositories.
Follow the license terms of each dataset.
