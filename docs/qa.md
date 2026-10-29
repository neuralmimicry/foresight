# QA and user acceptance

Unit tests cover the contract-specific invariants described in the README. The SwarmHPC executable UAT script exercises live HTTP authorization and PostgreSQL-backed behavior with uniquely named fixture records. Fixture evaluations stay fixture_only and cannot be promoted. Run it with private bearer-header files; it emits only assertion summaries.

Required release checks: formatting, Clippy with warnings denied, Rust tests, architecture build, Ansible syntax/lint/template checks, published image provenance, healthy rollout, API authorization, replay/conflict behavior, source revocation, temporal corrections, data-leakage rejection, fixture qualification rejection and persistence after restart.

Business acceptance remains separate: a permissioned real dataset and independently isolated candidate must beat the declared baseline under Gail's qualification policy; a real target adapter must demonstrate approval binding, one bounded effect, ambiguous-result reconciliation and a recorded observed outcome. Fixture UAT cannot satisfy those gates. Unsupported capabilities must remain explicit unavailable/abstain states.

