# Self-Hosting a Tycho Indexer

This document outlines the requirements, costs, and considerations for hosting your own Tycho Indexer instead of using Propellerheads' hosted service.

## Executive Summary

Self-hosting a Tycho Indexer is **technically feasible** as the project is open-source (MIT License) and well-documented. However, it requires:

- **Substreams API access** (via Pinax, The Graph, or StreamingFast)
- **RPC node access** (with trace support for certain protocols)
- **PostgreSQL database** with significant storage
- **Moderate infrastructure** for running the indexer service
- **DevOps expertise** to maintain and monitor the system

**Estimated Monthly Cost: $150 - $500+** depending on scale and providers.

---

## Architecture Overview

```
┌─────────────────────┐     ┌──────────────────────┐     ┌─────────────────┐
│   Blockchain Node   │────▶│  Substreams Provider │────▶│  Tycho Indexer  │
│   (Archive RPC)     │     │  (Pinax/The Graph)   │     │    Service      │
└─────────────────────┘     └──────────────────────┘     └────────┬────────┘
                                                                   │
                                      ┌────────────────────────────┼────────────────────────────┐
                                      │                            │                            │
                                      ▼                            ▼                            ▼
                            ┌─────────────────┐          ┌─────────────────┐         ┌──────────────────┐
                            │   PostgreSQL    │          │   WebSocket     │         │   RPC/HTTP API   │
                            │    Database     │          │  Subscriptions  │         │     Service      │
                            └─────────────────┘          └─────────────────┘         └──────────────────┘
```

Tycho Indexer consists of several components:
- **Extractors**: Process incoming blockchain data from Substreams
- **Storage**: PostgreSQL database for state persistence
- **WebSocket Service**: Real-time updates to clients
- **RPC Service**: REST API for querying historical/current state

---

## Prerequisites & Dependencies

### 1. Substreams API Access (REQUIRED)

Tycho uses **Substreams** (by StreamingFast) for high-performance blockchain indexing. You cannot self-host the Substreams infrastructure easily - you need to use a provider.

#### Providers

| Provider          | Free Tier             | Pro Tier            | Notes                            |
| ----------------- | --------------------- | ------------------- | -------------------------------- |
| **Pinax**         | $25/mo usage included | $49/mo + $200 usage | Recommended, supports 48+ chains |
| **The Graph**     | Varies                | Varies              | Part of decentralized network    |
| **StreamingFast** | Contact               | Contact             | Original creators                |

#### Pinax Pricing Details (Recommended)
- **Free Plan**: $25/mo in usage (~1TB Substreams data)
- **Pro Plan**: $49/mo + $200/mo usage included (~8TB data)
- **Additional Usage**: $25 per TB

#### How to Get API Token
1. Create account at [app.pinax.network](https://app.pinax.network/)
2. Generate API key in dashboard
3. Set as `SUBSTREAMS_API_TOKEN` environment variable

### 2. RPC Node Access (REQUIRED)

Tycho requires RPC access for:
- Fetching storage data for Dynamic Contract Indexing (DCI)
- Testing and validation
- Some protocol integrations

**Requirements:**
- **Archive node** with full historical state
- **Trace support** (`debug_traceCall`) for certain protocols

#### Options

| Provider                      | Archive + Traces | Cost Estimate        |
| ----------------------------- | ---------------- | -------------------- |
| **Pinax RPC**                 | Yes              | $6/1M requests       |
| **Alchemy**                   | Yes (Growth+)    | $49-199/mo           |
| **Infura**                    | Limited          | $50-225/mo           |
| **QuickNode**                 | Yes              | $49-299/mo           |
| **Chainstack**                | Yes              | $49-379/mo           |
| **Self-hosted (Erigon/Reth)** | Yes              | 2TB+ SSD + bandwidth |

### 3. PostgreSQL Database (REQUIRED)

Tycho stores all indexed state in PostgreSQL.

**Requirements:**
- PostgreSQL 13+ with `pg_cron` extension
- Sufficient storage for protocol state
- Fast SSD storage recommended

**Estimated Storage:**
- Ethereum mainnet (all protocols): 50-200GB+
- Per chain: 20-100GB depending on protocols indexed
- Historical data grows over time (configurable retention)

### 4. Substreams Packages (SPKGs)

These are pre-built packages that define what data to extract. Propellerheads maintains packages for:
- Uniswap V2/V3/V4
- SushiSwap V2
- PancakeSwap V2/V3
- Balancer V2
- Curve
- Ekubo V2
- And more...

SPKGs are included in the tycho-indexer repository.

---

## Infrastructure Requirements

### Minimum Specifications

| Component   | Minimum               | Recommended           |
| ----------- | --------------------- | --------------------- |
| **CPU**     | 4 cores               | 8+ cores              |
| **RAM**     | 8 GB                  | 16+ GB                |
| **Storage** | 100 GB SSD            | 500GB+ NVMe           |
| **Network** | 100 Mbps              | 1 Gbps                |
| **OS**      | Linux (Debian/Ubuntu) | Linux (Debian/Ubuntu) |

### Docker Requirements
- Docker 20.10+
- Docker Compose 2.x

---

## Setup Guide

### Step 1: Clone the Repository

```bash
git clone https://github.com/propeller-heads/tycho-indexer.git
cd tycho-indexer
```

### Step 2: Configure Environment

Create a `.env` file:

```bash
# Substreams Authentication (from Pinax or other provider)
SUBSTREAMS_API_TOKEN=your-substreams-api-token

# RPC URLs
RPC_URL=https://eth-mainnet.g.alchemy.com/v2/your-key
TRACE_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/your-key

# Database
DATABASE_URL=postgres://postgres:mypassword@db:5432/tycho_indexer_0

# Logging
RUST_LOG=info

# Optional: OpenTelemetry for observability
OTLP_EXPORTER_ENDPOINT=http://lgtm:4317
```

### Step 3: Configure Extractors

Edit `extractors.yaml` to specify which protocols to index:

```yaml
extractors:
  uniswap_v2:
    name: "uniswap_v2"
    chain: "ethereum"
    implementation_type: "Custom"
    sync_batch_size: 1000
    start_block: 10008300
    protocol_types:
      - name: "uniswap_v2_pool"
        financial_type: "Swap"
    spkg: "substreams/ethereum-uniswap-v2/ethereum-uniswap-v2-v0.3.0.spkg"
    module_name: "map_pool_events"
    
  uniswap_v3:
    name: "uniswap_v3"
    chain: "ethereum"
    # ... additional config
```

### Step 4: Start Services

```bash
# Start PostgreSQL and observability stack
docker compose up -d db lgtm promtail

# Wait for database to be ready
./wait-for-postgres.sh db

# Start the indexer
docker compose up tycho-indexer
```

### Step 5: Verify Operation

```bash
# Check health
curl http://localhost:4242/v1/health

# Check logs
docker compose logs -f tycho-indexer
```

---

## Running Commands

The tycho-indexer binary supports several commands:

```bash
# Run all extractors from extractors.yaml
tycho-indexer --endpoint https://mainnet.eth.streamingfast.io:443 index

# Run single extractor
tycho-indexer --endpoint https://mainnet.eth.streamingfast.io:443 run --extractor uniswap_v2

# Run only RPC server (no indexing)
tycho-indexer rpc

# Run token analyzer
tycho-indexer analyze-tokens
```

---

## Cost Breakdown

### Monthly Estimates (Ethereum Mainnet Only)

| Item                       | Low Estimate | High Estimate |
| -------------------------- | ------------ | ------------- |
| **Substreams (Pinax Pro)** | $49          | $249          |
| **RPC (Archive + Traces)** | $50          | $200          |
| **Server (Cloud VM)**      | $40          | $150          |
| **Database Storage**       | $10          | $50           |
| **Total**                  | **~$150/mo** | **~$650/mo**  |

### Multi-Chain Costs

For each additional chain (Base, Unichain, etc.):
- Additional Substreams usage: +$25-100/mo
- Additional RPC access: +$50-150/mo
- Additional storage: +$10-30/mo

---

## Operational Considerations

### 1. Initial Sync Time

Starting from scratch requires syncing from the start blocks of each protocol:
- Uniswap V2: Block 10,008,300 (May 2020)
- Uniswap V3: Block 12,369,621 (May 2021)
- Full sync: **Several hours to days** depending on protocols

### 2. Reorg Handling

Tycho handles chain reorganizations automatically using a Reorg Buffer. Unfinalized blocks are held in memory until confirmed.

### 3. Data Retention

Configure retention horizon to balance storage costs:
```bash
tycho-indexer index --retention-horizon 2025-07-23T00:00:00
```

### 4. Monitoring

The docker-compose includes Grafana LGTM stack:
- **Grafana**: http://localhost:3000 (admin/admin)
- **Loki**: Log aggregation
- **Tempo**: Distributed tracing
- View metrics, logs, and traces in Grafana

### 5. High Availability

For production:
- Run multiple indexer instances behind load balancer
- Use managed PostgreSQL (RDS, Cloud SQL)
- Implement proper backup strategies

---

## Comparison: Self-Hosted vs Hosted

| Aspect                  | Propellerheads Hosted            | Self-Hosted         |
| ----------------------- | -------------------------------- | ------------------- |
| **Setup Time**          | Minutes                          | Hours/Days          |
| **Maintenance**         | None                             | Significant         |
| **Cost (Single Chain)** | API Key (contact Propellerheads) | $150-500/mo         |
| **Control**             | Limited                          | Full                |
| **Latency**             | Variable                         | Optimizable         |
| **Multi-Chain**         | Easy                             | More infrastructure |
| **SLA**                 | Depends on agreement             | Self-managed        |
| **Customization**       | None                             | Full                |

---

## Alternatives to Consider

### 1. Request Dedicated Hosted Instance

Contact Propellerheads about dedicated hosting with SLA:
- Documentation: https://docs.propellerheads.xyz/tycho
- Telegram Community: https://t.me/+B4CNQwv7dgIyYTJl

### 2. Become a Tycho Fellow

From documentation:
> "We are working with independent third parties to host additional indexers, both free and paid – to add redundancy to our indexer."

Requirements:
- Integrate new DEXs
- Settle >$1M/week on Tycho Router

### 3. Hybrid Approach

- Use hosted service as primary
- Self-host as backup/fallback
- Switch between based on availability

---

## Resources

- **Repository**: https://github.com/propeller-heads/tycho-indexer
- **Documentation**: https://docs.propellerheads.xyz/tycho
- **Telegram Community**: https://t.me/+B4CNQwv7dgIyYTJl
- **Pinax Substreams**: https://pinax.network/products/substreams
- **Substreams Docs**: https://docs.substreams.dev/

---

## Conclusion

Self-hosting a Tycho Indexer is viable for organizations that:

1. **Need guaranteed uptime** and can't rely on third-party hosted service
2. **Have DevOps capacity** to maintain blockchain infrastructure
3. **Want customization** of indexing behavior or protocols
4. **Can justify the cost** ($150-500+/month per chain)

For most users, the hosted Propellerheads service remains the simpler option. Consider reaching out to them about reliability concerns or SLA agreements before committing to self-hosting.

---

*Document generated: December 2024*
*Tycho Indexer version referenced: 0.125.x*
