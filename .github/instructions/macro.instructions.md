---
applyTo: lib/macro/**
---
# Macro

Procedural macro crate at `lib/macro/`.

## Derive macros

| Macro | Input | Generated |
|---|---|---|
| `StateEnumDerive` | Enum named `*Value` with `(Id, Value)` variants | Corresponding `*Id` enum + `*Item` downcasting trait |
| `Id` | Enum or struct | `ext_id() -> ExternalId` + `TryFrom<ExternalId>` |
| `IdDelegation` | Enum wrapping multiple `Id` types | Delegates `ext_id()` to inner variants |
| `EnumVariants` | Enum | `variants()` method returning all possible values (incl. nested combinations) |

## StateEnumDerive naming convention

The enum **must** be named `*Value` (e.g., `DeviceStateValue`). The macro strips the `Value` suffix to generate the ID enum name (e.g., `DeviceStateId`).

