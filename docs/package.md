# Axiom package manifest

Phase 5 adds a small project-level package manifest in JSON:

- `axiom.pkg` (`MANIFEST_FILENAME`)

Current supported fields:

- `name` (required, string): Package name, used as artifact basename unless overridden.
- `version` (required, string): Package semantic version.
- `main` (optional, default: `src/main.ax`): Source file entrypoint, relative to project root.
- `out_dir` (optional, default: `dist`): Directory for compiled bytecode artifacts.
- `output` (optional, string): Custom output filename or path inside `out_dir`.
  - If omitted, output defaults to `<name>.axb`.
  - If it does not end with `.axb`, `.axb` is appended.

Example manifest:

```json
{
  "name": "demo",
  "version": "0.1.0",
  "main": "src/main.ax",
  "out_dir": "dist",
  "output": "artifact.axb"
}
```

## CLI

```bash
python -m axiom pkg init /path/to/project --name demo
python -m axiom pkg build /path/to/project
```

`pkg init` creates:

- `axiom.pkg` if missing
- `src/main.ax` (only if missing) with `print 0` fallback body

`pkg build` reads `axiom.pkg`, compiles `main`, and writes `.axb` into `out_dir`.
