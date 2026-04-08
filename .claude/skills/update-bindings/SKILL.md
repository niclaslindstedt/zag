---
description: "Use when the user wants to keep the six language bindings (TypeScript, Python, C#, Swift, Java, Kotlin) in sync with the Rust AgentBuilder source of truth. Guides adding new builder options, CLI flags, tests, and README updates across all bindings."
---

# Updating Language Bindings

The six language bindings (TypeScript, Python, C#, Swift, Java, Kotlin) mirror the Rust `AgentBuilder` via CLI subprocess calls. Each binding has a `ZagBuilder` class that constructs CLI arguments and spawns the `zag` binary. They tend to fall out of sync when new builder options are added to Rust but not propagated to all bindings.

## Upstream References

- **Rust AgentBuilder (source of truth)**: `zag-agent/src/builder.rs` — canonical builder fields (lines 49-72) and setter methods
- **CLI flags**: `zag-cli/src/cli.rs` — `AgentArgs`, `SessionIsolationArgs`, exec-specific args
- **CLI wiring**: `zag-cli/src/commands/agent_action.rs` — how CLI args map to agent configuration

## Discovery Process

1. Read `zag-agent/src/builder.rs` (fields on `AgentBuilder` struct, lines 49-72) to get the current Rust builder fields
2. For each binding, read the builder file and list all setter methods
3. Compare: identify methods present in Rust but missing from any binding, and vice versa
4. Check `zag-cli/src/cli.rs` for any new CLI flags not yet wired into bindings
5. Note: `progress` (`Box<dyn ProgressHandler>`) is Rust-only — intentionally excluded from bindings
6. Note: `bin()` and `debug()` exist in all six bindings but NOT in the Rust `AgentBuilder` (binding-only)
7. Note: Swift has additional binding-only methods: `connection()`, `remote()`, `urlSession()` for remote mode

## Automated Discovery

Compare Rust builder setter methods against each binding:

```sh
# Rust builder setters
grep 'pub fn ' zag-agent/src/builder.rs | head -30

# TypeScript
grep -E '^\s+\w+\(.*\).*: this' bindings/typescript/src/builder.ts

# Python
grep -E '^\s+def \w+\(self' bindings/python/src/zag/builder.py

# C#
grep -E 'public ZagBuilder \w+\(' bindings/csharp/src/Zag/ZagBuilder.cs

# Swift
grep -E 'public func \w+' bindings/swift/Sources/Zag/ZagBuilder.swift

# Java
grep -E 'public ZagBuilder \w+\(' bindings/java/src/main/java/io/zag/ZagBuilder.java

# Kotlin
grep -E 'fun \w+\(' bindings/kotlin/src/main/kotlin/zag/ZagBuilder.kt
```

## Implementation Files

### Primary — Source of truth

| File | Role |
|------|------|
| `zag-agent/src/builder.rs` | Rust AgentBuilder — canonical field list and setter methods |
| `zag-cli/src/cli.rs` | CLI flag definitions (AgentArgs, SessionIsolationArgs, exec args) |

### Primary — Binding builders

| Language | Builder | Tests | README | Reference |
|----------|---------|-------|--------|-----------|
| TypeScript | `bindings/typescript/src/builder.ts` | `bindings/typescript/tests/builder.test.ts` | `bindings/typescript/README.md` | `bindings/typescript/REFERENCE.md` |
| Python | `bindings/python/src/zag/builder.py` | `bindings/python/tests/test_builder.py` | `bindings/python/README.md` | `bindings/python/REFERENCE.md` |
| C# | `bindings/csharp/src/Zag/ZagBuilder.cs` | `bindings/csharp/tests/Zag.Tests/ZagBuilderTests.cs` | `bindings/csharp/README.md` | `bindings/csharp/REFERENCE.md` |
| Swift | `bindings/swift/Sources/Zag/ZagBuilder.swift` | `bindings/swift/Tests/ZagTests/ZagBuilderTests.swift` | `bindings/swift/README.md` | `bindings/swift/REFERENCE.md` |
| Java | `bindings/java/src/main/java/io/zag/ZagBuilder.java` | `bindings/java/src/test/java/io/zag/ZagBuilderTests.java` | `bindings/java/README.md` | `bindings/java/REFERENCE.md` |
| Kotlin | `bindings/kotlin/src/main/kotlin/zag/ZagBuilder.kt` | `bindings/kotlin/src/test/kotlin/zag/ZagBuilderTests.kt` | `bindings/kotlin/README.md` | `bindings/kotlin/REFERENCE.md` |

### Secondary (only when adding new capabilities)

- `zag-cli/src/commands/agent_action.rs` — wiring CLI args to agent configuration
- `zag-agent/src/builder_tests.rs` — Rust builder tests

## Implementation Patterns

### Architecture

Each binding follows the same architecture:

1. A `ZagBuilder` class with private fields mirroring Rust's `AgentBuilder`
2. Fluent setter methods that return `self`/`this` for chaining
3. Two internal arg-building methods: `buildGlobalArgs()` and `buildExecArgs()`
4. Terminal methods (`exec`, `stream`, `execStreaming`, `run`, `resume`, `continueLast`) that spawn the CLI

### Naming conventions by language

| Concept | TypeScript | Python | C# | Swift | Java | Kotlin |
|---------|-----------|--------|-----|-------|------|--------|
| Method style | `camelCase` | `snake_case` | `PascalCase` | `camelCase` | `camelCase` | `camelCase` |
| Bool default | `(v = true)` | `(v: bool = True)` | `(bool v = true)` | `()` no param | overloads: `()` + `(boolean)` | `(v: Boolean = true)` |
| Return type | `: this` | `-> ZagBuilder` | `ZagBuilder` | `-> Self` | `ZagBuilder` | `= apply { }` |
| Async exec | `async exec()` | `async def exec()` | `async Task<> ExecAsync()` | `async func exec()` | `exec() throws` (sync) | `suspend fun exec()` |

### Global args vs exec args

**Global args** go in `buildGlobalArgs()` — placed before the subcommand:

`-p/--provider`, `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir`, `-w/--worktree`, `--sandbox`, `--verbose`, `--quiet`, `--debug`, `--session`, `--max-turns`, `--show-usage`, `--size`

**Exec args** go in `buildExecArgs()` — placed after the `exec` subcommand:

`--json`, `--json-schema`, `--json-stream`, `-o/--output`, `-i/--input-format`, `--replay-user-messages`, `--include-partial-messages`

### Worktree and sandbox pattern

Worktree and sandbox support both unnamed (flag-only) and named variants. Each language handles optional names differently:

- **TypeScript**: `_worktree: string | boolean`; check `=== true` vs `typeof === "string"`
- **Python**: `_worktree: str | bool | None`; check `is True` vs `isinstance(str)`
- **C#**: `object?` holding `true` or a `string`; pattern-match with `is true` / `is string s`
- **Swift**: `IsolationOption` enum with `.enabled` and `.named(String)` cases
- **Java**: `Object` holding `Boolean.TRUE` or `String`; `instanceof` checks
- **Kotlin**: `Any?` holding `true` or `String`; `when` expression

### Default output format

All bindings default to `-o json` for non-streaming `exec()` calls (when no explicit output format or json-stream is set). This ensures structured `AgentOutput` parsing.

### Binding-only methods

These exist in all six bindings but NOT in the Rust `AgentBuilder`:

- `bin(path)` — override CLI binary path (default: `ZAG_BIN` env var or `"zag"`)
- `debug(flag)` — maps to `--debug` global CLI flag

Swift has additional binding-only methods for remote `zag serve` mode:

- `connection(ZagConnection)` — configure a remote connection
- `remote(url:token:)` — convenience for remote connection
- `urlSession(URLSession)` — custom URLSession for testing

## Adding a New Builder Option

When a new field is added to `AgentBuilder` in `builder.rs`, propagate to all six bindings:

### Step 1: Determine flag placement

- Is it a global flag (before subcommand)? → `buildGlobalArgs()`
- Is it exec-specific (after `exec`)? → `buildExecArgs()`

### Step 2: TypeScript (`bindings/typescript/src/builder.ts`)

```typescript
// 1. Field (with other private fields)
private _newOption?: string;

// 2. Setter
/** Description. */
newOption(value: string): this {
  this._newOption = value;
  return this;
}

// 3. In buildGlobalArgs() or buildExecArgs()
if (this._newOption) args.push("--new-option", this._newOption);
```

### Step 3: Python (`bindings/python/src/zag/builder.py`)

```python
# 1. Field in __init__
self._new_option: str | None = None

# 2. Setter
def new_option(self, value: str) -> ZagBuilder:
    """Description."""
    self._new_option = value
    return self

# 3. In _global_args() or _exec_args()
if self._new_option:
    args.extend(["--new-option", self._new_option])
```

### Step 4: C# (`bindings/csharp/src/Zag/ZagBuilder.cs`)

```csharp
// 1. Field
private string? _newOption;

// 2. Setter
/// <summary>Description.</summary>
public ZagBuilder NewOption(string value) { _newOption = value; return this; }

// 3. In BuildGlobalArgs() or BuildExecArgs()
if (_newOption != null) { args.Add("--new-option"); args.Add(_newOption); }
```

### Step 5: Swift (`bindings/swift/Sources/Zag/ZagBuilder.swift`)

```swift
// 1. Field
private var _newOption: String?

// 2. Setter
@discardableResult
public func newOption(_ value: String) -> Self { _newOption = value; return self }

// 3. In buildGlobalArgs() or buildExecArgs()
if let v = _newOption { args += ["--new-option", v] }
```

### Step 6: Java (`bindings/java/src/main/java/io/zag/ZagBuilder.java`)

```java
// 1. Field
private String newOption;

// 2. Setter (add no-arg + boolean overloads for flag-type options)
public ZagBuilder newOption(String value) { this.newOption = value; return this; }

// 3. In buildGlobalArgs() or buildExecArgs()
if (newOption != null) { args.add("--new-option"); args.add(newOption); }
```

### Step 7: Kotlin (`bindings/kotlin/src/main/kotlin/zag/ZagBuilder.kt`)

```kotlin
// 1. Field
private var _newOption: String? = null

// 2. Setter
fun newOption(value: String) = apply { _newOption = value }

// 3. In buildGlobalArgs() or buildExecArgs()
_newOption?.let { args.addAll(listOf("--new-option", it)) }
```

### Step 8: Tests

Add the new method to the builder chaining test in each binding's test file. At minimum, verify:
- The setter chains correctly (returns builder)
- The arg appears in the built args list

### Step 9: READMEs and REFERENCE.md

Add a row to the "Builder methods" table in each binding's README:

```markdown
| `.newOption(value)` | Description of what it does |
```

Follow each language's naming convention for the method name.

Also update the corresponding `REFERENCE.md` in each binding directory. The REFERENCE.md contains the full API signature (with types), CLI flag mapping, and must stay in sync with the builder. Add the new method to the Configuration Methods table with its full signature, CLI flag, and description.

## Update Checklist

- [ ] Add field and setter to Rust `AgentBuilder` in `zag-agent/src/builder.rs`
- [ ] Wire into `create_agent()` in `builder.rs`
- [ ] Add CLI flag to `zag-cli/src/cli.rs` if user-facing
- [ ] Wire in `zag-cli/src/commands/agent_action.rs` if needed
- [ ] Add Rust tests in `zag-agent/src/builder_tests.rs`
- [ ] **TypeScript**: field + setter + arg builder in `builder.ts`, test in `builder.test.ts`, README row, REFERENCE.md
- [ ] **Python**: field + setter + arg builder in `builder.py`, test in `test_builder.py`, README row, REFERENCE.md
- [ ] **C#**: field + setter + arg builder in `ZagBuilder.cs`, test in `ZagBuilderTests.cs`, README row, REFERENCE.md
- [ ] **Swift**: field + setter + arg builder in `ZagBuilder.swift`, test in `ZagBuilderTests.swift`, README row, REFERENCE.md
- [ ] **Java**: field + setter + arg builder in `ZagBuilder.java`, test in `ZagBuilderTests.java`, README row, REFERENCE.md
- [ ] **Kotlin**: field + setter + arg builder in `ZagBuilder.kt`, test in `ZagBuilderTests.kt`, README row, REFERENCE.md

## Verification

```sh
# Rust
make build && make test && make clippy

# TypeScript
cd bindings/typescript && npm run build && npm test

# Python
cd bindings/python && python -m pytest

# C#
cd bindings/csharp && dotnet test

# Swift
cd bindings/swift && swift test

# Java
cd bindings/java && mvn test

# Kotlin
cd bindings/kotlin && gradle test
```
