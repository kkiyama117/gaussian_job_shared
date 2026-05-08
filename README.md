# gaussian_job_shared

GAUSSIANジョブパイプラインの共有データ型ライブラリ。京都大学スパコンのSLURM投入を念頭に、ログインノード側 (バッチ生成 / 投入 / CLI) と計算ノード側 (実行ランタイム) の両方から参照される **Rust** クレートです。Python向けには [PyO3](https://pyo3.rs/) + [maturin](https://www.maturin.rs/) で `gaussian_job_shared._core` を公開します (現状は最小スタブ)。

> 詳細設計: [`docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`](docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md)

## 主要な型 (Rust)

`entities` は **2 つの階層**で整理されています:

- `entities::workflow` — フロー視点 (DAG ノード / ライフサイクル状態 / フロー全体)。SLURM 設定を *使う*が、SLURM 内部の概念ではない。
- `entities::slurm` — sbatch ディレクティブ素材と `SlurmJobConfig` エンベロープ。

### `entities::workflow`

`JobSpec` (small / 状態非依存) と `Job` (large / フロー内) の **2 段構成** + DAG (`JobFlow`) です。

| 型 | 役割 |
|----|------|
| `JobFlow` | 1 つの論理ジョブフロー単位 (UUID v7, `calc_type`, `work_dir`, `tags`, `jobs: BTreeMap<JobId, Job>`)。`BTreeMap` の構造自体が `JobId` の一意性とソート順を担保 |
| `Job` | フロー内に置かれた `JobSpec` + 入辺 (`parents: Vec<JobEdge>`)。将来の実行時状態 (`slurm_jobid` / `status_history` 等) の拡張点 |
| `JobSpec` | `program` + `config: SlurmJobConfig` + `body` (bash 本文)。フロー非依存で複数フロー間で再利用可能 |
| `JobId` / `Program` / `CalcType` | 透過 (`#[serde(transparent)]`) ニュータイプ |
| `JobEdge` | `Job.parents` に積む入辺。`from: JobId` + `kind: DependencyType` (afterok / afterany / after / …) |
| `JobLifecycleStatus` | `queued` / `running` / `done` / `failed` — Python 側 `Status` と対応するワークフロー視点の状態 (SLURM の `PENDING/RUNNING/...` とは別概念) |
| `StatusEntry` | `(status, transitioned_at: DateTime<Utc>)` のペア |

### `entities::slurm` (SLURM 用フィールド型)

- `SlurmJobConfig` — `partition` / `time_limit` / `log_stdout` / `log_stderr` / `comment` / `job_name` / `array_spec` / `dependency` / `mail_user` / `mail_types` / `resource_spec`
- `JobTimeLimit` — `--time` の 6 種表記 (`M`, `M:S`, `H:M:S`, `D-H`, `D-H:M`, `D-H:M:S`) を受け、常に `HH:MM:SS` で再シリアライズ
- `ResourceSpec` (`p=…:t=…:c=…:m=…` / `g=…`) — 京大スパコンの `--rsc` 形式
- `SlurmDependency` (`afterok:…`, `afterany:…` 等) と `SlurmArraySpec` (`--array=0-9%2` 等)
- `MailType` / `MailTypeInput`

### `error`

- `SchemaParseError` (TOML スキーマ違反: 未知キー / 必須欠落 / パース失敗)
- `SLURMJOBError`

## 設定ファイル例

`examples/common.toml` と `examples/experiment.toml` に注釈付きサンプルがあります。

## インストール

### Rust (Cargo)

```toml
[dependencies]
gaussian_job_shared = { git = "https://github.com/miyake-ken/gaussian-job-shared.git", branch = "main" }
```

### Python (uv)

```toml
[project]
dependencies = [
  "gaussian_job_shared @ git+https://github.com/miyake-ken/gaussian-job-shared.git@main",
]
```

### Python (Pixi)

GAUSSIAN_repo ワークスペースの常用配信チャネル:

```toml
[pypi-dependencies]
gaussian_job_shared = { git = "https://github.com/miyake-ken/gaussian-job-shared.git", branch = "main" }
```

## 開発

### Rust 側

```bash
cargo fmt
cargo clippy -- -D warnings
cargo check
cargo test
```

`rust-toolchain.toml` で **nightly** に固定 (`rustfmt`, `rust-src`)。

### Python (PyO3 / maturin) 側

```bash
uv sync
uv run maturin develop                # _core を in-place ビルド
uv run pytest -q
uv run ruff check .
uv run ruff format --check .
```

### Cargo features

- `pyo3` (default) — `pyo3` / `pyo3-async-runtimes` / `pyo3-log` / `pythonize` を有効化
- `stub_gen` (default) — `pyo3-stub-gen` 経由で `.pyi` を生成する `stub_gen` バイナリを有効化

`pyo3/extension-module` は **default に含めず**、各 maturin パッケージの `pyproject.toml` 側 (`[tool.maturin] features`) で個別に有効化する方針です。これにより `stub_gen` バイナリが libpython にリンクできます。
