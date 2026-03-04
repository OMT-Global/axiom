# Axiom package manifest

Phase 5 adds a small project-level package manifest in JSON:

- `axiom.pkg` (`MANIFEST_FILENAME`)

Current supported fields:

- `name` (required, string): Package name, used as artifact basename unless overridden.
- `version` (required, string): Package semantic version.
- `main` (optional, default: `src/main.ax`): Source file entrypoint, relative to project root.
- `out_dir` (optional, default: `dist`): Directory for compiled bytecode artifacts.
- `output` (optional, string): Custom output filename or path inside `out_dir`.
  - Must be a relative path and may not traverse parent directories (no `..`).
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
python -m axiom pkg init /path/to/project --name demo --version 0.1.0
python -m axiom pkg build /path/to/project
python -m axiom pkg manifest /path/to/project
python -m axiom pkg check /path/to/project
python -m axiom pkg run /path/to/project
```

`pkg init` creates:

- `axiom.pkg` (with default values)
- `src/main.ax` (only if missing) with `print 0` fallback body
Options:

- `--name`
- `--version` (default `0.1.0`)
- `--main` (default `src/main.ax`)
- `--out-dir` (default `dist`)
- `--output` (optional explicit artifact filename)
- `--force` to regenerate `axiom.pkg` when it already exists.

`pkg build` reads `axiom.pkg`, compiles `main`, and writes `.axb` into `out_dir`.

`pkg check` validates `axiom.pkg` and compiles the manifest `main` entrypoint.
Host side-effecting host calls (for example `host.print`) obey the global
`--allow-host-side-effects` flag.

`pkg run` reads `axiom.pkg`, compiles `main`, and executes it in the VM immediately.

`pkg manifest` prints normalized manifest JSON.

`pkg clean` removes the configured `out_dir` directory entirely.
