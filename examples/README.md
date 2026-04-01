# Examples

This directory contains sample input specs and pre-generated output in every
format. The two input files exercise every combination of severity and change
type across paths, parameters, and schemas.

## Input files

| File | Description |
|------|-------------|
| `base.yaml` | The "before" OpenAPI spec (v1.0.0) |
| `changed.yaml` | The "after" OpenAPI spec (v2.0.0) |

## Output samples

| File | Format |
|------|--------|
| `showcase.txt` | Plain text (default) |
| `showcase.md` | Markdown |
| `showcase.json` | JSON |
| `showcase.html` | HTML |

## Reproducing

Build `osd` first, then run any of the commands below from the repository root:

```
cargo build --release
```

### Text (default)

```
./target/release/osd diff examples/base.yaml examples/changed.yaml
```

### Markdown

```
./target/release/osd diff examples/base.yaml examples/changed.yaml -f markdown
```

### JSON

```
./target/release/osd diff examples/base.yaml examples/changed.yaml -f json
```

### HTML

```
./target/release/osd diff examples/base.yaml examples/changed.yaml -f html
```

### Writing to a file

Add `-o` to write to a file instead of stdout:

```
./target/release/osd diff examples/base.yaml examples/changed.yaml -f html -o report.html
```

### Regenerating all samples

```
./target/release/osd diff examples/base.yaml examples/changed.yaml -f text     -o examples/showcase.txt
./target/release/osd diff examples/base.yaml examples/changed.yaml -f markdown -o examples/showcase.md
./target/release/osd diff examples/base.yaml examples/changed.yaml -f json     -o examples/showcase.json
./target/release/osd diff examples/base.yaml examples/changed.yaml -f html     -o examples/showcase.html
```
