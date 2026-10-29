# Retention interface

POST /v1/retention/purge accepts the source reference under a control grant. Relay must first attest a current deletion tombstone for that source object and actor. The product removes mutable derived context, predictions, reports and outcomes; Relay clears retained change payloads. Minimal action identities, actor, content digest and receipt state survive so deletion cannot enable a duplicate external write. Source systems, Refiner traces/conversations and PostgreSQL backups retain separate policy owners. No purge endpoint claims to erase those external stores.

