# Canonical wire contract registry

`registry.json` is the reviewable cross-language registry metadata for P1.9 wire contracts. It owns contract version, layer/protocol, fixture location, requiredness/nullability, enum constraints, timestamp/number/unit metadata, and compatibility aliases. It is **not** itself a standalone JSON Schema document; `schema_dialect` records the intended validation dialect for consumers, while the registry remains metadata that the language-specific tests interpret.

`registry_version` versions the registry artifact itself. Each contract has an independent integer `version`. Application/library versions do not change contract versions. A breaking wire change increments the contract version; compatible additions may keep the version when existing readers remain valid.

Fixtures under `fixtures/` are canonical emitted JSON examples. Producers may accept documented N-1 aliases, but canonical output must use registry field names and enum casing. For example, the current Rust owner emits `WifiSecretAction` as `Keep`/`Set`/`Clear`; lowercase values are compatibility input only.

Validation is intentionally split by consumer language:

- `hydragrow-shared/tests/p1_9_schema_registry.rs` validates the registry and round-trips representative Rust payloads.
- `hydragrow-frontend/src/contracts/sharedSchemaRegistry.test.ts` validates the same registry and fixtures at the TypeScript transport boundary.

The fixture corpus is the shared input to both test suites. Do not create a second editable TypeScript/Rust schema copy for these contracts.
