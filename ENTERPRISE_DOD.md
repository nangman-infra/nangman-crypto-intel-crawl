# intel-crawl-app Enterprise DoD

작성일: 2026-05-07 KST

이 문서는 `intel-crawl-app` 하나만의 완료 조건을 고정한다.

## scope

포함:

```text
RSS/REST/static HTML public source fetch
raw_intel_event JSONL chunk upload to AWS S3
source-local, cross-source content, and near-duplicate dedup
source_health JSONL chunk upload to AWS S3
source_heal JSONL chunk upload to AWS S3
source_coverage JSONL chunk upload to AWS S3
source_balance JSONL chunk upload to AWS S3
source-fetch-state, dedup-index, and manifest upload to AWS S3
JetStream-acknowledged NATS pointer publish
Docker Compose on-prem worker operation
```

제외:

```text
strategy judgment
candidate generation
LLM/NLP/NLI analysis
community firehose
private API
login/cookie/browser automation
order placement
live trading
```

## DoD checklist

### 1. Contract correctness

```text
source registry JSON loads successfully
universe asset count is 50
enabled source_id is unique
enabled direct_asset source must reference only assets in the 50-asset universe
available-but-disabled sources may exist for community/noise-budget or outside-universe reasons
enabled fetch_method is one of rss/rest_api/html_crawl
rest_api source must use an explicit allowlisted adapter
source_url must use https
unknown --source-id must fail instead of producing an empty successful run
```

### 2. Data quality

```text
common-word symbols require explicit ticker context
direct asset sources cannot mark non-universe assets as top50 relevant
market reaction community RSS sources must be present in the registry even when disabled
disabled community RSS sources must carry source_state and activation_blocker
enabled community sources must have a per-source item cap
derivatives REST sources must have run and per-source caps
derivatives REST sources must apply the enterprise safety ceiling even if env values are higher
derivatives REST asset requests must prioritize asset-specific verified symbols before global-news-only symbols
raw_intel_event must carry content/source quality metadata for INTEL-L1 routing
dedup_key prevents repeated S3/NATS emission across repeated runs
dedup-index-v2 preserves canonical URL, normalized content hash, and SimHash metadata
conditional GET state prevents unchanged public sources from being reprocessed every run
source failures are recorded per source without hiding the failure in the summary
```

### 3. Delivery durability

```text
raw_intel_event is uploaded to AWS S3 before NATS publish
raw_intel_event_created_v2 carries bucket/key/line_number/byte_offset/byte_length
one raw event per object is forbidden; raw events are written as JSONL chunks
source coverage diagnostics are written once per run
source balance diagnostics are written once per run
NATS publish uses JetStream publish, not Core fire-and-forget publish
publish uses expected stream RAW_INTEL
publish waits for server ack before incrementing events_published
NATS message id is stable per raw_intel_event
NATS publish failure after S3 upload writes publish-outbox/status=pending
NATS publish success writes publish-outbox/status=published
pending outbox replay republishes pointers without deleting or rewriting raw S3 objects
```

### 4. Operations

```text
container runs as non-root uid 10001
runtime output is AWS S3 object storage, not local bind-mounted files
container drops Linux capabilities
container uses no-new-privileges
container root filesystem is read-only
Compose healthcheck verifies runtime certificate readability
NATS server remains a separate app and external Docker network dependency
AWS S3 storage remains outside the app container
intel-crawl-app has no DeleteObject runtime permission
```

### 5. Verification gate

Required local commands:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run -- --dry-run --max-items-per-source 2
sudo docker compose -f /opt/nangman-crypto/intel-crawl/compose.yml --env-file /opt/nangman-crypto/intel-crawl/.env config
sudo docker compose -f /opt/nangman-crypto/intel-crawl/compose.yml --env-file /opt/nangman-crypto/intel-crawl/.env ps
sudo docker run --rm --network host natsio/nats-box:0.17.0 nats --server nats://127.0.0.1:4222 stream info RAW_INTEL
```

## current verdict

```text
DoD is code-defined and testable.
Final completion verdict requires all verification gate commands to pass after deployment.
```

## external basis

NATS JetStream publish is required because JetStream publish calls return a
server acknowledgment after persistence. Core publish alone is not sufficient for
this app's enterprise delivery DoD.

Reference:

```text
https://docs.nats.io/using-nats/developer/develop_jetstream
```
