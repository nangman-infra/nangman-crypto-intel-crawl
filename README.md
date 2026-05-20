# intel-crawl-app

Raw intel crawler for RSS, exchange notices, and low-frequency public context.

## Production Contract

- Repository: `git@github.com:nangman-infra/nangman-crypto-intel-crawl.git`
- Runtime role: L0 raw intelligence collector for AI-DLC alpha discovery.
- Default deployment shape: one long-running compose/ECS worker.
- State contract: stateless compute; durable state is object-store JSONL,
  manifest, dedup-index, and optional NATS JetStream publish acknowledgments.
- Default source registry:
  `/opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json`
- Forbidden boundary: no private API, login cookie, browser automation, model
  call, strategy judgment, order placement, or live trading.

This app is intentionally narrow:

```text
source-registry.rss-seed.v1.json
  -> enabled rss/rest_api/html_crawl sources
  -> raw_intel_event JSONL chunks in RustFS
  -> source-health/source-heal JSONL chunks in RustFS
  -> dedup-index and crawl manifest in RustFS
  -> raw_intel_event.created JetStream-acknowledged NATS storage pointer
```

It does not use private APIs, browser automation, login cookies, community
firehose collection, strategy judgment, candidate generation, model calls, order
placement, or live trading.

Implemented fetch methods:

```text
rss        -> RSS or Atom feeds
rest_api   -> allowlisted public JSON adapters
html_crawl -> static public HTML anchor extraction
```

The source registry tracks both active and available-but-disabled sources. This
keeps market reaction/community sources discoverable without forcing noisy
community firehose collection into the default worker loop.

The worker also writes source coverage and source balance diagnostics. Coverage
is per major-50 asset, so downstream work can see whether a symbol has only
global news coverage or real asset-specific sources. Balance diagnostics make
derivatives and community caps visible instead of silently dropping noisy input.

Implemented REST adapters:

```text
binance_cms_announcement_list
binance_usdm_funding_rate_latest
binance_usdm_funding_rate_history
binance_usdm_open_interest
```

`binance_usdm_funding_rate_history` is a manual backfill adapter. It is not part
of the default live worker source set; select it explicitly with a source id and
an audited time window:

```bash
cargo run \
  -- \
  --dry-run \
  --source-id derivatives_binance_usdm_funding_rate_history_rest \
  --backfill-start-ms 1764892800000 \
  --backfill-end-ms 1764979200000 \
  --max-items-per-source 1000
```

Backfill raw events carry:

```text
historical_source_depth
backfill_window_start_ms
backfill_window_end_ms
source_time_range_verified
```

NATS is not embedded in this app container. Start the shared on-prem NATS server
from:

```text
/opt/nangman-crypto/nats-server
```

Default registry:

```text
/opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json
```

RustFS bucket:

```text
intel-crawl-app-l0
```

RustFS object layout:

```text
raw-intel-events/schema=raw_intel_event_v1/dt=YYYY-MM-DD/hour=HH/source_category=.../source_id=.../run_id=.../part-000001.jsonl
source-health/schema=source_health_v1/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
source-heal/schema=source_heal_event_v1/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
source-coverage/schema=source_coverage_v1/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
source-balance/schema=source_balance_v1/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
dedup-index/schema=dedup_index_v1/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
publish-outbox/status=published/schema=raw_intel_event_created_v2/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
publish-outbox/status=pending/schema=raw_intel_event_created_v2/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
manifests/schema=intel_l0_manifest_v1/dt=YYYY-MM-DD/hour=HH/run_id=....json
```

Before writing a new event, the worker loads recent RustFS `dedup-index`
chunks. Repeated runs skip already-written events and only publish NATS pointers
for newly stored events.

When NATS publishing is enabled, the worker uses JetStream publish with expected
stream `RAW_INTEL` and waits for the server publish acknowledgment before
counting an event as published. The NATS message id is the stable
`raw_intel_event` id. The published payload is `raw_intel_event_created_v2` and
contains a `storage_ref` pointing at a RustFS JSONL record.

Symbol matching is intentionally conservative for common English words. Assets
such as `NOT`, `NEAR`, `TON`, `BIO`, `CHIP`, `DASH`, `DOGS`, `HIVE`, `MEGA`, and
`TRUMP` require explicit ticker context like `NOTUSDT`, `NOT-USDT`, `$NOT`, or
`(NOT)` instead of a bare word match.

Raw events include quality metadata for INTEL-L1 routing:

```text
content_kind
content_quality
content_quality_score
source_quality
source_relevance_scope
direct_asset_count
matched_asset_count
```

Default flood controls:

```text
INTEL_CRAWL_DERIVATIVES_MAX_EVENTS_PER_RUN=12
INTEL_CRAWL_DERIVATIVES_MAX_EVENTS_PER_SOURCE=6
INTEL_CRAWL_COMMUNITY_MAX_EVENTS_PER_RUN=30
INTEL_CRAWL_COMMUNITY_MAX_EVENTS_PER_SOURCE=5
```

Live derivatives REST sources are also clamped by an enterprise safety ceiling of 12
events per run and 6 events per source. Manual funding history backfill sources
are selected explicitly and are not treated as the live worker loop. The worker prioritizes assets with
`rss_seed_status=asset_specific_verified` before global-news-only assets so
low-signal numeric snapshots do not dominate INTEL-L1.

Run a dry check:

```bash
cargo run \
  -- \
  --dry-run \
  --max-items-per-source 2
```

Write RustFS JSONL chunks:

```bash
AWS_ACCESS_KEY_ID=... \
AWS_SECRET_ACCESS_KEY=... \
cargo run \
  -- \
  --object-store-endpoint https://s3.nangman.cloud \
  --object-store-bucket intel-crawl-app-l0 \
  --object-store-region us-east-1 \
  --object-store-force-path-style true
```

Publish RustFS-backed created pointers to NATS:

```bash
AWS_ACCESS_KEY_ID=... \
AWS_SECRET_ACCESS_KEY=... \
cargo run \
  -- \
  --object-store-endpoint https://s3.nangman.cloud \
  --object-store-bucket intel-crawl-app-l0 \
  --object-store-region us-east-1 \
  --object-store-force-path-style true \
  --nats-url nats://127.0.0.1:4222 \
  --nats-subject raw_intel_event.created \
  --nats-stream RAW_INTEL
```

Run as a compose-managed worker:

```bash
cd /opt/nangman-crypto/intel-crawl
scripts/setup-host.sh
scripts/deploy.sh
```

Enterprise completion criteria are tracked in:

```text
/opt/nangman-crypto/intel-crawl/ENTERPRISE_DOD.md
```
