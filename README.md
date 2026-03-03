# rust_macro_internal

`rust_macro_internal` is an internal procedural macro crate in the `CLR` workspace.

It provides compile-time code generation driven by source artifacts (INI, CSV, JSON, and Mermaid files), and is used as the implementation layer behind higher-level macro workflows.

## Direction

`rust_macro_internal` is moving toward a broader **diagram-to-code** vision: using structured, human-readable diagrams as the authoritative source of truth for generated Rust code.

Rather than writing boilerplate by hand, the goal is to express intent visually — state machines, packet layouts, call flows, data schemas — and have the macro layer synthesize correct, idiomatic Rust from those artifacts at compile time.

This positions the crate as an exploration of what the Rust ecosystem could gain from first-class diagram-driven codegen: fewer hand-maintained structs, tighter coupling between documentation and implementation, and a path toward making complex system designs directly executable.

Mermaid is the current substrate for this work, with INI, CSV, and JSON as supporting formats for data-oriented generation. The longer-term direction is richer diagram semantics — sequence flows becoming async function scaffolds, state diagrams becoming typestate machines, packet diagrams becoming zero-copy layout structs — all driven by the same compile-time macro machinery.

## Location

- Crate path: `CLR/rust_macro_internal`
- Workspace root: `CLR/Cargo.toml`

## Crate type

This crate is a `proc-macro` library:

```toml
[lib]
proc-macro = true
```

## Main capabilities

- Generate enums/structs from INI files
- Generate structs/enums/lookups from CSV files
- Generate structs from JSON files
- Generate state/flow-based code from Mermaid diagrams
- Generate packet-layout structs and bit-vector helpers
- Generate function scaffolding from sequence diagrams

## Macro inventory

### INI macros

- `#[ini_enum("path/to/file.ini")]`
- `#[ini_struct("path/to/file.ini")]`
- `#[ini_enum_str("path/to/file.ini", "SomeTag")]`

### CSV macros

- `#[csv_struct("path/to/file.csv")]`
- `#[csv_struct2("path/to/file.csv", "SomeTag")]`
- `#[csv2enum_variants("path/to/file.csv", TagIdent)]`
- `csv2lookup!("path/to/file.csv", KeyCol, ValueCol, EnumName)`
- `#[csv2enum_lookup("path/to/file.csv", EnumIdent, "ValueCol")]`
- `csv2hash!("path/to/file.csv", KeyCol, ValueCol)`

### JSON macros

- `json_struct!("path/to/file.json", TypeName)`
- `json_struct2!("path/to/file.json", "RootName", TypeName)`

### Mermaid flow/state macros

- `#[flow_enum("path/to/flow.mmd")]`
- `#[state_struct("path/to/state.mmd")]`
- `#[state_struct_trait("path/to/state.mmd")]`
- `state_type_mapping!("path/to/state.mmd", TypeTag)`

### Mermaid packet/sequence macros

- `#[packet_struct("path/to/packet.mmd")]`
- `#[packet_bit_vec("path/to/packet.mmd")]`
- `#[sequence2function("path/to/sequence.mmd")]`

## Notes on arguments

Most macros parse arguments as:

- `"path"` (string literal)
- optional extra identifiers (`Tag`, `EnumName`, `TypeName`, etc.)
- some forms accept a second string-literal tag

The parser for macro arguments is implemented in `src/init_args.rs`.

## Example usage

```rust
use rust_macro_internal::{json_struct, csv2hash};

json_struct!("test_data/spec.json", ConfigRoot);

const LOOKUP: std::sync::LazyLock<std::collections::HashMap<String, String>> =
    std::sync::LazyLock::new(|| csv2hash!("test_data/table.csv", Key, Value));
```

```rust
use rust_macro_internal::{ini_struct, state_struct};

#[ini_struct("test_data/settings.ini")]
pub struct Settings;

#[state_struct("test_data/state_diagram.mmd")]
pub struct MachineState;
```

## Build and check

From `CLR/`:

```bash
cargo check -p rust_macro_internal
cargo test -p rust_macro_internal
```

## Internal crate notice

`rust_macro_internal` is intended for workspace/internal use, and macro signatures may evolve with parser/codegen changes.
