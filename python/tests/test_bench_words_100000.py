import daachorse
import ahocorasick_rs

from tests import dataset


def run_daachorse(pma, haystack):
    pma.find_overlapping(haystack)


def run_ahocorasick_rs(pma, haystack):
    pma.find_matches_as_indexes(haystack, True)


def test_daachorse_bytewise_words_100000_bench(benchmark):
    patterns = [pattern.encode() for pattern in dataset.load_words_100000()]
    haystack = dataset.load_sherlock().encode()
    pma = daachorse.DoubleArrayAhoCorasick(patterns)
    benchmark(run_daachorse, pma, haystack)


def test_daachorse_charwise_words_100000_bench(benchmark):
    patterns = dataset.load_words_100000()
    haystack = dataset.load_sherlock()
    pma = daachorse.CharwiseDoubleArrayAhoCorasick(patterns)
    benchmark(run_daachorse, pma, haystack)


def test_ahocorasick_rs_words_100000_bench(benchmark):
    patterns = dataset.load_words_100000()
    haystack = dataset.load_sherlock()
    pma = ahocorasick_rs.AhoCorasick(patterns)
    benchmark(run_ahocorasick_rs, pma, haystack)
