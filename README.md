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
The bundled registry currently enables 42 public sources, including 16
asset-specific developer/governance/project feeds for `AAVE`, `ADA`, `AVAX`,
`BCH`, `BTC`, `DOGE`, `ETH`, `LINK`, `LTC`, `NEAR`, `SOL`, `SUI`, `TON`, `TRX`,
`UNI`, and `ZEC`.

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

NATS remains a pointer bus, not the canonical store. The crawler publishes only
after the raw JSONL record is written to object storage. The expected on-prem
JetStream stream contract is:

```text
stream: RAW_INTEL
subject: raw_intel_event.created
payload schema: raw_intel_event_created_v2
message id: raw_intel_event.event_id
duplicate window: at least 120 seconds
storage: file
retention: limits
```

Smoke-check a reachable on-prem NATS endpoint without writing crawler objects:

```bash
cd /Volumes/WD/Developments/nangman-crypto/apps/intel-crawl-app
NATS_SMOKE_URL=nats://127.0.0.1:4222 scripts/nats-smoke.sh
```

The smoke test creates or reuses a small `RAW_INTEL_SMOKE` JetStream stream,
publishes the same stable message id twice, waits for publish acknowledgments,
and verifies that JetStream keeps one message.

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
dedup-index-v2/schema=dedup_index_v2/dt=YYYY-MM-DD/hash_prefix=.../hour=HH/run_id=.../part-000001.jsonl
source-fetch-state/schema=source_fetch_state_v1/source_id=.../state.json
publish-outbox/status=published/schema=raw_intel_event_created_v2/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
publish-outbox/status=pending/schema=raw_intel_event_created_v2/dt=YYYY-MM-DD/hour=HH/run_id=.../part-000001.jsonl
manifests/schema=intel_l0_manifest_v1/dt=YYYY-MM-DD/hour=HH/run_id=....json
```

AWS dev deployment:

```text
Account: 791444962214
Region: ap-northeast-2
Profile: AdministratorAccess-791444962214
ECR repository: ecr-nangman-dev-intel-crawl-apn2
ECS cluster: ecs-nangman-dev-invest-apn2
ECS service: svc-nangman-dev-intel-crawl
ECS task definition: td-nangman-dev-intel-crawl
ECS container: intel-crawl
ECS capacity provider: FARGATE_SPOT
ECS task size: 256 CPU / 512 MiB memory
CloudWatch log group: /ecs/nangman/dev/intel-crawl
S3 L0 bucket: nangman-crypto-dev-intel-crawl-l0-962214
IAM execution role: role-nangman-dev-intel-crawl-exec
IAM task role: role-nangman-dev-intel-crawl-task
```

The AWS dev worker uses AWS S3 as the L0 object store:

```bash
cd /Volumes/WD/Developments/nangman-crypto/apps/intel-crawl-app

docker buildx build \
  --platform linux/arm64 \
  --provenance=false \
  --sbom=false \
  -t 791444962214.dkr.ecr.ap-northeast-2.amazonaws.com/ecr-nangman-dev-intel-crawl-apn2:git-$(git rev-parse --short=12 HEAD)-arm64-single \
  --push \
  /Volumes/WD/Developments/nangman-crypto/apps/intel-crawl-app
```

The ECS task command is S3-first and publishes optional NATS pointers after raw
objects are stored. AWS dev currently uses the on-prem NATS endpoint reachable
through the VPN route to `192.168.10.0/24`:

```text
--object-store-endpoint https://s3.ap-northeast-2.amazonaws.com
--object-store-bucket nangman-crypto-dev-intel-crawl-l0-962214
--object-store-region ap-northeast-2
--object-store-force-path-style false
--schedule-interval-ms 900000
--max-items-per-source 20
--nats-url nats://192.168.10.45:4222
--nats-subject raw_intel_event.created
--nats-stream RAW_INTEL
```

Operate and verify the AWS worker with separate checks instead of treating
`RUNNING` as healthy:

```bash
aws ecs describe-services \
  --cluster ecs-nangman-dev-invest-apn2 \
  --services svc-nangman-dev-intel-crawl \
  --profile AdministratorAccess-791444962214 \
  --region ap-northeast-2

aws ecs list-tasks \
  --cluster ecs-nangman-dev-invest-apn2 \
  --service-name svc-nangman-dev-intel-crawl \
  --profile AdministratorAccess-791444962214 \
  --region ap-northeast-2

aws logs describe-log-streams \
  --log-group-name /ecs/nangman/dev/intel-crawl \
  --order-by LastEventTime \
  --descending \
  --max-items 5 \
  --profile AdministratorAccess-791444962214 \
  --region ap-northeast-2

aws s3api list-objects-v2 \
  --bucket nangman-crypto-dev-intel-crawl-l0-962214 \
  --prefix manifests/schema=intel_l0_manifest_v1/ \
  --profile AdministratorAccess-791444962214 \
  --region ap-northeast-2

aws logs filter-log-events \
  --log-group-name /ecs/nangman/dev/intel-crawl \
  --filter-pattern 'panic ?ERROR ?OutOfMemory ?SIGKILL ?Killed ?AccessDenied' \
  --profile AdministratorAccess-791444962214 \
  --region ap-northeast-2
```

Current dev deployment notes:

```text
GitHub Actions: Quality Checks and SonarQube Scan are required on main.
Sonar issue count must be zero, not only quality-gate passing.

ECS service: run on FARGATE_SPOT because the worker is stateless and S3 is the
durable source of truth. Use desiredCount=1 for the dev crawler and keep
capacityProviderStrategy=[{capacityProvider=FARGATE_SPOT, weight=1, base=0}].

Task size: start at 256 CPU / 512 MiB memory. Recent dev CloudWatch metrics on
the previous 512 CPU / 1024 MiB task showed low utilization, so increase only
after CPU, memory, timeout, or OOM evidence.

Runtime image: use a nonroot distroless runtime image. The app does not need a
shell or package manager at runtime; public source fetching and S3 writes are
handled by the static app process plus CA certificates.
```

Before writing a new event, the worker loads recent RustFS `dedup-index`
compatibility chunks and the candidate-specific `dedup-index-v2` hash-prefix
shards. Repeated runs skip already-written events and only publish NATS pointers
for newly stored events. `dedup-index-v2` stores source identity, canonical URL,
normalized content hash, and SimHash metadata so the worker can suppress exact,
cross-source content, and near-duplicate events before they reach INTEL-L1.

The worker also persists `source-fetch-state` for each source. RSS, static HTML,
and supported REST list endpoints reuse stored `ETag` and `Last-Modified`
headers with conditional GET requests; `304 Not Modified` is recorded as healthy
source activity without reprocessing unchanged content. Consecutive failures set
a bounded source-level backoff so unstable sources do not dominate a Spot worker.

After raw events are successfully persisted to S3, those stored events are
included in `dedup-index` even if optional NATS pointer publish is pending. This
keeps Spot restarts or temporary NATS outages from writing the same raw event
again; pending pointer delivery is tracked separately through
`publish-outbox/status=pending`.

When NATS publishing is enabled, the worker uses JetStream publish with expected
stream `RAW_INTEL` and waits for the server publish acknowledgment before
counting an event as published. The NATS message id is the stable
`raw_intel_event` id. The published payload is `raw_intel_event_created_v2` and
contains a `storage_ref` pointing at a RustFS JSONL record.

Use the smoke test before enabling NATS in a long-running worker:

```bash
NATS_SMOKE_URL=nats://nats.internal:4222 \
NATS_SMOKE_STREAM=RAW_INTEL_SMOKE \
NATS_SMOKE_SUBJECT=raw_intel_event.created.smoke \
scripts/nats-smoke.sh
```

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

The live loop also applies source cadence gates from persisted fetch state:

```text
high   -> at most once per 15 minutes
medium -> at most once per 30 minutes
low    -> at most once per 6 hours
```

Manual `--source-id` and audited backfill windows bypass the cadence gate so
operator checks do not get hidden by scheduled-loop throttling.

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

Replay pending NATS pointer outbox records without deleting or rewriting S3 raw
events:

```bash
AWS_ACCESS_KEY_ID=... \
AWS_SECRET_ACCESS_KEY=... \
cargo run \
  -- \
  --replay-pending-outbox \
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
