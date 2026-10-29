# Operations

The Ansible business-suite role provisions a separate PostgreSQL database and non-superuser role per product, private registry pull credentials, a ClusterIP service, digest-pinned Deployment, security context, probes, bounded resources and a NetworkPolicy. Continuum remains the apply/rollout owner through the existing continuum_tenant_k8s_app role. New public ingress is not enabled by default.

Secrets live in controller .secrets/business-suite/<host> and Kubernetes Secrets; keep them outside Git. A read-only deploy key permits CI to fetch the shared Verity kernel. Rotate upstream, database and Customers tokens independently. Never pass tokens in URLs or capture them in verification reports.

Before release: run Rust quality gates and the suite's functional UAT against isolated state. Build ARM64 images for the current cluster, record source and OCI digests, and supply the image map to Ansible. The image CI also publishes AMD64 builds for development; deploy only a digest for the intended architecture.

After release: verify rollout, available replicas, exact resolved imageID, /readyz, unauthenticated API rejection and the authenticated UAT. Run the playbook again to check convergence. Record unavailable dependencies separately from failed product behavior. Qualification and action effects require task-specific acceptance.

Rollback: redeploy the preceding digest map with the same playbook. Database v1 creates one backward-compatible table and index; do not drop state during an image rollback. Pause new writes for a database restore, restore the product database from the existing PostgreSQL backup policy into a separate database, compare cursor counts and immutable IDs, then cut over DATABASE_URL through Ansible. Reconcile Relay uncertain receipts before enabling writes. Never mark an unknown remote write successful to make a rollout pass.

Retention/export: use authorised bounded APIs for context/evidence exports. Relay tombstones remove Cora's local context when imported and deny source access immediately. Source systems, Refiner conversation/trace storage and PostgreSQL backups have independent deletion/retention owners; include those stores in a deletion request. There is no automatic cross-product backup purge in v0.1.0.

