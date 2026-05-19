BLOOM FILTER STATISTICAL TEST DOCUMENTATION

==============================================================================
THEORETICAL FALSE POSITIVE RATE CALCULATION
==============================================================================

For a Bloom filter with:
  - m = 4096 bits (BLOOM_BITS)
  - k = 2 hash functions (NUM_HASH_FUNCTIONS)
  - n = number of elements inserted

The theoretical false positive rate is:
  p = (1 - (1 - 1/m)^(k*n))^k

For our typical fixture files with ~630 trigrams per file:
  - n ≈ 630
  - k = 2
  - p ≈ (1 - (1 - 1/4096)^1260)^2 ≈ 0.069 or 6.9%

This is the probability that a randomly chosen trigram that was NOT inserted
will be incorrectly reported as present. This probability increases as more
trigrams are inserted into the filter (filter saturation).

==============================================================================
WHY EMPIRICAL BOUNDS ARE 15% MEAN AND 30% MAX
==============================================================================

The synthetic fixture files and probe trigrams in the tests introduce variance:

1. Fixture Generation: The 1000 synthetic Rust source files are deterministically
   generated from formulas, so they exhibit patterns that may differ from truly
   random source code. This can affect collision rates.

2. Probe Trigrams: The synthetic absent trigrams (using byte patterns like
   [0xFF, 0xFE, 0xFD]) are not distributed uniformly across trigram space.
   Some may collide with source-like content more than others.

3. Empirical Measurement: The measured false positive rate varies between files
   because different files have different trigram distributions. Some filters
   may be more saturated than others.

The empirical bounds of 15% mean and 30% max provide:
  - Headroom above the theoretical ~7% for measurement variance
  - Confidence that the filter is still effective at rejecting non-matching files
  - A safety margin without being so loose as to hide filter degradation

If empirical rates exceed these bounds, it indicates either:
  - Fixture generation is creating pathological inputs
  - Hash function collisions are worse than expected
  - The filter parameters (4096 bits, 2 hash functions) need tuning

==============================================================================
WHY TEST 8 (STATISTICS REPORT) ALWAYS PASSES
==============================================================================

This test is documentation, not a correctness assertion. It always passes
because its only purpose is to print statistics to stdout for human review.

The test enables visibility into:
  - How many trigrams are actually inserted across the test corpus
  - The bit saturation level (how full the filters become)
  - How the estimated FPR (computed from bit count) compares to empirical rates
  - Whether false negative violations occur (they should always be 0)

If false negative violations were detected during this test's measurements,
the test would still pass but the count printed would be non-zero, alerting
the developer to investigate further.

This pattern is common in statistical testing frameworks where measurement
and documentation take priority over strict pass/fail assertions.

==============================================================================
WHY SIEVE REJECTION RATE LOWER BOUND IS 80%
==============================================================================

When search terms are deliberately absent from a file, the Bloom filter should
reject them (return false from probably_contains) for most queries. Why not 100%?

1. False Positives are Inherent: Bloom filters have a non-zero false positive
   rate by design. For ~630-trigram filters, ~7% of absent trigrams will
   incorrectly pass the filter check.

2. Test Construction: The test uses search terms derived from file index
   offsets (+500 offset) to find terms that are unlikely to appear. However,
   due to the deterministic formula-based file generation, some cross-file
   similarity can occur.

3. Sieve Logic: The sieve uses OR semantics (should_parse if ANY query's
   trigrams are present). With a 7% FPR, about 7% of rejections will become
   false acceptances (parse calls when not strictly necessary).

The 80% lower bound ensures:
  - The sieve is filtering out the majority of non-matching files
  - Each file in the test corpus still has meaningful reject opportunities
  - The rejection rate is sufficiently high to validate the sieve's value

A rejection rate below 80% would indicate either:
  - Test terms are too similar to the fixture files
  - The hash functions are producing collisions
  - The fixture generation creates unexpected patterns

==============================================================================
TEST DESCRIPTIONS AND INVARIANTS
==============================================================================

Test 1: Zero False Negatives (1000 files)
  - Invariant: If a trigram was inserted, probably_contains MUST return true
  - Failure means: Bug in insert() or probably_contains() logic
  - Expected result: All assertions pass, zero violations detected

Test 2: False Positive Rate Within Bounds
  - Measures rate against synthetic absent trigrams
  - Asserts mean < 15% and max < 30%
  - Failure means: Filter saturation is higher than expected, or fixtures create collisions

Test 3: False Positive Rate Increases With Saturation
  - Asserts FPR is monotonically non-decreasing as trigrams are inserted
  - Failure means: Hash function or insertion logic has a bug

Test 4: Estimate Correlates With Empirical Rate
  - Measures Pearson correlation between false_positive_estimate() and measured rates
  - Asserts correlation > 0.5 (moderate to strong)
  - Failure means: The estimate formula is not predictive of actual behavior

Test 5: Sieve Rejection Correctness
  - Tests the full sieve pipeline end-to-end
  - Asserts zero false negatives (should_parse returns true for matching terms)
  - Asserts rejection rate > 80% for non-matching terms
  - Failure means: Sieve integration or query parsing has a bug

Test 6: Bit Count Correctness
  - Verifies bit_count() is accurate and within logical bounds
  - Asserts bit_count() <= BLOOM_BITS and <= 2 * inserted_trigram_count
  - Failure means: bit_count() or bit manipulation logic is incorrect

Test 7: Serialization Preserves Properties
  - Round-trips filters through to_bytes() / from_bytes()
  - Verifies FPR estimate, bit count, and query results are identical
  - Failure means: Serialization introduces data loss or corruption

Test 8: Statistics Report
  - Prints aggregate statistics across all 1000 files
  - Always passes (documentation only)
  - Shows: total trigrams, mean saturation, FPR comparisons, zero false negatives

==============================================================================
HOW TO INTERPRET TEST OUTPUT
==============================================================================

Example output line from Test 8:
  empirical mean FPR:          11.40%

This means: When we probe a file's filter with trigrams that were NOT inserted,
the filter incorrectly returns true (false positive) for 11.40% of probes on
average across all 1000 files.

This is higher than theoretical (~7%) because:
  - The synthetic fixture files have patterns not purely random
  - Probe trigrams may cluster in high-collision regions of hash space
  - Measurement variance with only 200 probes per filter

This is acceptable because:
  - It's still well below the 15% empirical bound
  - For 630 trigrams per filter, a 7% false positive rate means the filter
    still rejects ~530 out of ~600 probe trigrams per file on average
  - This rejection rate is sufficient for the sieve to be effective

==============================================================================
FIXTURE GENERATION STRATEGY
==============================================================================

The 1000 synthetic Rust files are generated deterministically using:
  - File index i (0-999) as seed
  - Line index j (0 to 50-200) within file
  - Template selection based on (i ^ j) % 7 to vary syntax
  - Identifier generation using format!("sym_{}_{}", i*31 + j, i ^ j)

Example file 0, line 0:
  fn sym_0_0() {}

This ensures:
  - Identical files generated on every test run (reproducibility)
  - Sufficient variety in trigrams across 1000 files
  - Deterministic offsets for constructing "absent" terms (+500 offset)
  - No filesystem I/O (hermetic test)
  - Fast execution (no I/O overhead)

==============================================================================
