# gaussian_job_shared

Shared structures between A (login-node batch generation, SLURM submission, CLI) and B (compute-node runtime) of the GAUSSIAN job pipeline. This package is Group **D** in the A/B/C/D package division.

## Modules

- `gaussian_job_shared.config` — `read_common_toml`, `read_experiment_toml`; strict-schema TOML loaders.
- `gaussian_job_shared.dataclasses` — frozen models (`CommonConfig`, `Slurm`, `SlurmPost`, `ResourceSpec`, `Env`, `GaussianCmd`, `ExperimentDoc`, `NormalizedStep`, `Compounds`, `Metadata`, `CalcBlock`, `CalcParams`, `GaussianParams`) plus the `(program, calc_type)` registry (`register`, `get_params_class`, `known_programs`, `calc_types_for`).
- `gaussian_job_shared.paths` — `PathResolver` with 7 UUID-keyed accessors (`target_dir`, `temp_dir`, `metadata_path`, `status_path`, `input_dir`, `output_dir`, `derived_dir`).
- `gaussian_job_shared.fs` — atomic IO helpers: `read_metadata` / `write_metadata` / `update_jobids` (metadata.toml round-trip), `read_status` / `write_status` (lifecycle flag), `prepare_inputs` / `copy_results` (additive directory copy).

## Configuration files

This version uses **two** TOML files:

### `common.toml` — cluster/account-level settings (rarely changes)

```toml
[slurm]
partition   = "gr10641a"
job_name    = "GAUSSIAN"
time_limit  = "48:00:00"
log_stdout  = "/x/%x.%j.out"
log_stderr  = "/x/%x.%j.err"
mail_user   = "u@example.com"
mail_types  = ["BEGIN", "END", "FAIL"]

[slurm.resource_spec]
p = 1
t = 56
c = 56
m = "56G"

[env]
root          = "/LARGE0/gr10641/calc_data/GAUSSIAN"
tmp_root      = "/tmp/gaussian"
task_basename = "main"

[gaussian_cmd]
command = "g16"
```

### `experiment.toml` — what to compute (per experiment)

```toml
[experiment]
id   = "exp_demo"
tags = { project = "donor_acceptor_screen" }

[[step]]
program      = "gaussian"
calc_type    = "opt"
compounds    = ["X"]
parent_uuids = []
tags         = { basis = "6-31g(d)" }

[step.params]
route        = "#p opt b3lyp/6-31g(d)"
charge       = 0
multiplicity = 1
extra_input  = ""
```

## Usage

```python
from gaussian_job_shared import (
    read_common_toml,
    read_experiment_toml,
    PathResolver,
)

common = read_common_toml("common.toml")
exp    = read_experiment_toml("experiment.toml")
paths  = PathResolver(common.env)

# For each step you assign a UUID v7 (E does this in production):
# metadata_path = paths.metadata_path(uuid_v7_string)
```

## Install

### uv

```toml
[dependencies]
gaussian_job_shared = { git = "https://github.com/miyake-ken/gaussian-job-shared.git", branch = "main" }
```

### Pixi
Consumed via a pinned git URL in pixi (the steady-state distribution channel for the GAUSSIAN_repo workspace):

```toml
[pypi-dependencies]
gaussian_job_shared = { git = "https://github.com/miyake-ken/gaussian-job-shared.git", branch = "main" }
```

## Development

```bash
uv sync                          # uses [dependency-groups] dev (pytest etc.)
uv run pytest -q
uv run ruff check .
uv run ruff format --check .
```
