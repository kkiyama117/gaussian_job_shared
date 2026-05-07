# gaussian_job_shared

Shared structure for running tasks in SLURM submission (in Kyoto Univ super computer)

## API

This library includes some structures and functions like below.

- errors
  - 

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
