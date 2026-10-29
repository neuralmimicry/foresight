# NeuralMimicry Foresight

Business numerical tasks and capacity scenarios over Gail.

Foresight owns feature/target semantics, time cutoffs, task definitions, provenance and scenario assumptions. Gail owns numerical models, fitting, artefacts, qualification and bounded inference. Verity supplies exact evidence envelopes; both its envelope and Gail's independent admission gates must permit a prediction.

POST /v1/predictions supports compute_demand in cpu_cores and job_duration in seconds, initially scoped to the capacity domain. Inputs include immutable model/dataset/evaluation identities, a versioned feature schema with explicit names and units, observation timestamps, cutoff, horizon and expiry. Feature values observed after the cutoff or older than one hour are rejected. Current source permissions come from Relay. Source read access never grants training rights.

The service fetches GET Verity /v1/evaluations/{id}, checks candidate/data/features/task/tenant/domain/expiry, then calls Gail /v1/numerical/predict. Gail's response must identify the same model and offer an admitted finite prediction and nonnegative uncertainty. Otherwise the durable record explicitly abstains. GET /v1/predictions/{id} rechecks current source rights. POST /v1/scenarios compares an unexpired compute-demand prediction with explicit capacity/reservation assumptions; it grants no infrastructure authority and makes no causal claim.

There is no generic classification/ranking/anomaly API, automatic training, model registry, causal engine or constrained optimiser in this initial release. A running deployment does not imply a qualified business model. A missing Gail endpoint, disabled ELM, missing qualification or unavailable source cannot produce a prediction.

## Build and run

Rust 1.98.1 or newer, PostgreSQL 16 or newer. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` (add `--workspace` in Verity). Lockfiles are committed. The shared kernel is a pinned Git dependency from neuralmimicry/verity. CI receives a read-only Verity deploy key through VERITY_READ_KEY. Never replace it with an unrestricted personal access token.

Set DATABASE_URL to a dedicated least-privilege product database, CUSTOMERS_URL to Customers, NM_TENANT to the single configured identity realm (default neuralmimicry), and NM_LISTEN (default 0.0.0.0:8080). Service URLs use <SERVICE>_URL; GAIL_TOKEN and CONDUCTOR_TOKEN are scoped upstream credentials. Caller bearer tokens are only forwarded to Customers, Relay, Verity and Refiner as required. TLS certificate verification is enabled and redirects are disabled. PostgreSQL uses an internal network connection; require cluster network isolation or a trusted TLS proxy for databases across trust boundaries.

`cargo run --release` starts the service. GET /healthz checks the process; GET /readyz checks its database. Business APIs require a live Customers observe/use/control grant for this service. Source operations also require Relay observe access and current source-purpose rights. The service never trusts forwarded identity headers or treats service-account tokens as human users. Each deployment is a single tenant realm; use separate databases and deployments for different identity realms.

HTTP bodies are limited to 1 MiB, upstream responses to 2 MiB, concurrent requests to 64, the database pool to eight and CPU evaluations to four. Requests have a 30-second timeout; CPU jobs retain their permits after caller cancellation. Product databases store mutable governed content; do not put their raw payloads in an immutable ledger. Logs avoid tokens and request bodies.

## Operations and acceptance

The deployment owner is SwarmHPC. Use `swarmhpc/ansible/continuum_tenant_foresight_site.yml`, or `continuum_tenant_business_suite_site.yml` for all four. See [operations](docs/operations.md) and [QA](docs/qa.md). Production evidence must record source commit, image digest, rollout/readiness, functional checks and limitations. A healthy pod does not qualify a model or prove an external business effect.

