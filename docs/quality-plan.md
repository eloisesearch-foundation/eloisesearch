# Quality, relevance and benchmark plan

How we prove that Eloisesearch is correct, returns relevant results, stays
reliable and performs, before claiming any of it.

## 1. Functional baseline first

Behavior is captured as executable fixtures (`contracts/search-cases.json` in
`eloisesearch-integration`) over a frozen, deterministic movie corpus. For each
case we record:

- the request and the relevant index configuration;
- the expected matching document IDs;
- the expected relative order where ranking matters;
- required response fields, pagination and highlight fragments;
- the error for invalid or unsupported input.

Cases cover plain queries, phrases, exclusions, the AND operator, prefix and
fuzzy terms, filters, browse ordering, facets, suggestions, accent folding and
malformed or incomplete input. Exact scores are never asserted.

## 2. Relevance evaluation

- Keep a small judged query set with the results a user would expect.
- Include title-only matches, overview-only matches, title-versus-overview
  trade-offs, filtered queries, no-result queries, punctuation and multi-term
  queries.
- Check that field weighting expresses the intended product behavior.
- Compare top results and ordering against the judgments, and across Eloisesearch
  versions to catch regressions.
- With enough judgments, report Precision@k, Recall@k and nDCG@k. For a small
  hand-checked corpus, a query-by-query checklist is more honest than weak
  aggregate numbers.

## 3. Performance and resources

Benchmark only after behavior is stable, with the same fixtures, queries, hardware,
concurrency and warm/cold conditions for every version or configuration compared.

| Dimension | Measure |
| --- | --- |
| Indexing | Documents per second, bulk size, time to first searchable result |
| Search latency | p50, p95, p99 per query class |
| Throughput | Requests per second at a defined concurrency |
| Memory | Idle, indexing peak, steady state |
| Disk | Index size and growth after updates, deletes and merges |
| Startup and recovery | Time to open an index and recover after restart |
| Freshness | Delay between an acknowledged write and its visibility |
| Stability | Error rate and latency under sustained load |

Record machine, OS, versions, build profile, index settings, dataset size, query
set, repetitions and the exact scripts. Keep microbenchmarks separate from
end-to-end HTTP measurements. Publish raw results and regressions as well as
improvements; results are versioned evidence, not permanent truths.

## 4. Reliability and lifecycle tests

The suite must cover:

- Create an index; index, retrieve, update and delete documents.
- Behavior after graceful restart and after abrupt termination.
- Write visibility under each `refresh` mode.
- Duplicate IDs, missing IDs, invalid schemas, malformed queries.
- Pagination boundaries and deterministic tie-breaking.
- Partial failure in bulk writes.
- Every error matches the documented error model.
- All of the above run in CI without external services.

## 5. Release gate

A release candidate needs all of the following. Progress is tracked in the
[roadmap](https://github.com/eloisesearch-foundation/eloisesearch-workspace/blob/main/docs/ROADMAP-MVP.md).

- All contract tests passing.
- Relevance reviewed against the judged set.
- Restart and lifecycle tests passing.
- API docs and the feature matrix current, including unsupported features.
- A reproducible benchmark report with environment details.
- Known limitations and operational requirements documented.
