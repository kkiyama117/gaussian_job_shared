# gaussian_job_shared

> **English summary** —
> `gaussian_job_shared` (a.k.a. `gaussian-job-shared2`) is the **workflow-tier**
> data-type library for the GAUSSIAN job pipeline on Kyoto University's HPC
> cluster. It owns the program-agnostic DAG view (`JobFlow` / `Job` /
> `JobSpec` / `JobEdge` / `JobId` / `Program` / `CalcType`) used by both the
> login-node side (batch generation / sbatch submission / CLI) and the
> compute-node side (run-time entry points). The **SLURM vocabulary**
> (`SlurmJobConfig`, `JobStatus`, `ResourceSpec`, `JobTimeLimit`, …) was
> extracted into the [`slurm_async_runner`](https://github.com/kkiyama117/slurm-async-runner)
> (SAR) crate; this crate consumes the Rust types only and re-uses SAR's
> pyclasses verbatim from the Python side via duck-typed `FromPyObject`
> bridges (the *Pyclass Single Owner* rule).

GAUSSIANジョブパイプラインの **ワークフロー層** 共有データ型ライブラリです。京都大学スパコンの SLURM 投入を念頭に、ログインノード側 (バッチ生成 / 投入 / CLI) と計算ノード側 (実行ランタイム) の両方から参照される **Rust** クレートで、Python 向けには [PyO3](https://pyo3.rs/) + [maturin](https://www.maturin.rs/) で `gaussian_job_shared._core` を公開します。

> 詳細設計:
> - [`docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`](docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md) — `JobFlow` / `Job` / `JobSpec` の設計
> - [`docs/superpowers/specs/2026-05-08-rust-python-ffi-design.md`](docs/superpowers/specs/2026-05-08-rust-python-ffi-design.md) — pyo3 ラッパーの設計

## アーキテクチャ概観

`gaussian_job_shared` は **2 層構成** のうち **ワークフロー層** だけを所有し、SLURM 語彙層は外部クレートから借りる形を取ります。

```
            ┌────────────────────────────────────────────────────┐
            │ slurm_async_runner (SAR)                            │
            │   entities::slurm::sbatch_options::*                │
            │     SlurmJobConfig / JobTimeLimit / ResourceSpec /  │
            │     SlurmDependency / SlurmArraySpec / Memory / ... │
            │   entities::slurm::status::*                        │
            │     JobStatus / JobState / JobReason                │
            │   ※ Python pyclass の Single Owner                  │
            └──────────────┬──────────────────────────────────────┘
                           │ Rust types のみ依存 (default-features = false)
                           ▼
            ┌────────────────────────────────────────────────────┐
            │ gaussian_job_shared (this crate)                    │
            │   entities::workflow::*                             │
            │     JobFlow / Job / JobSpec / JobEdge /             │
            │     JobId / Program / CalcType                      │
            │   py_export::bridge::*  ← duck-typed FromPyObject   │
            └────────────────────────────────────────────────────┘
```

**Pyclass Single Owner ルール** (SAR の設計ドキュメント由来):

- SAR が SLURM 語彙の `#[pyclass]` 実装を **唯一保持**。`gaussian_job_shared` の `cdylib` には SAR の `pyclass` シンボルをリンクしません (`default-features = false` で除外)。
- `gaussian_job_shared` の pyclass が SAR 型を引数に取る箇所は、`#[repr(transparent)]` newtype の `FromPyObject` 実装で Python 属性をダックタイプで読みます (詳細: [`src/py_export/bridge.rs`](src/py_export/bridge.rs))。
- 戻り値で SAR 型を返す箇所は、ランタイムに `Py::import` で SAR の正準クラスをロードして直接インスタンス化します。

## 主要な型 (Rust)

`entities` 直下に置かれているモジュールは **`workflow` のみ**。SLURM 語彙が必要な場合は `slurm_async_runner::entities::slurm::*` を import してください。

### `entities::workflow`

`JobSpec` (small / 状態非依存) と `Job` (large / フロー内) の **2 段構成** + DAG (`JobFlow`):

| 型 | 役割 |
|----|------|
| `JobFlow` | 1 つの論理ジョブフロー単位 (UUID v7, `created_at`, `tags`, `jobs: BTreeMap<JobId, Job>`)。`BTreeMap` の構造自体が `JobId` の一意性とソート順を担保 |
| `Job` | フロー内に置かれた `JobSpec` + 入辺 (`parents: Vec<JobEdge>`)。将来の実行時状態 (`slurm_jobid` / `status: Option<JobStatus>` 等) の拡張点 |
| `JobSpec` | `program` + `config: SlurmJobConfig` (SAR 型) + `body` (bash 本文)。フロー非依存で複数フロー間で再利用可能 |
| `JobId` / `Program` / `CalcType` | 透過 (`#[serde(transparent)]`) ニュータイプ |
| `JobEdge` | `Job.parents` に積む入辺。`from: JobId` + `kind: DependencyType` (afterok / afterany / after / …)。`DependencyType` は SAR から借用 |

### SLURM 語彙 (= SAR から借用)

歴史的経緯として、これらの型は以前 `gaussian_job_shared::entities::slurm` に居ましたが、`refactor!: consume slurm vocab from slurm_async_runner, drop local subtree` で SAR に集約されました。Rust から使う際は SAR 側のパスで参照してください。

```rust
use slurm_async_runner::entities::slurm::{
    SlurmJobConfig, JobTimeLimit, ResourceSpec, DependencyType,
    JobStatus, JobState, JobReason,
};
```

### `error`

- `SchemaParseError` (TOML スキーマ違反: 未知キー / 必須欠落 / パース失敗)
- `SLURMJOBError`

## Python サーフェス

`pip install` / `maturin develop` 後、Python からは以下のように import します。**ワークフロー型のみ** がこの wheel から提供され、SLURM 型は SAR の wheel から提供される点に注意してください。

```python
# Workflow types — this crate
from gaussian_job_shared._core.entities.workflow import (
    JobFlow, Job, JobSpec, JobEdge, JobId, Program, CalcType,
)

# SLURM vocab — from slurm_async_runner (SAR)
from slurm_async_runner._slurm_async_runner_core.entities.slurm.sbatch_options import (
    SlurmJobConfig, JobTimeLimit, ResourceSpec, DependencyType,
)
from slurm_async_runner._slurm_async_runner_core.entities.slurm.status import (
    JobStatus, JobState, JobReason,
)
```

`JobSpec(program=..., config=..., body=...)` の `config` 引数や `JobEdge(from_=..., kind=...)` の `kind` 引数には、SAR から取得した Python オブジェクトをそのまま渡せます (内部でブリッジが属性ベースで extract)。

> **注意:** `SlurmJobConfig` の `array_spec` / `dependency` / `mail_types` フィールドは現状ブリッジで `NotImplementedError` を投げるため、必要な場合は SAR 側で完成させた値を `SlurmJobConfig` 内に格納し、`gaussian_job_shared` の pyclass コンストラクタは経由しない経路を選ぶか、フィールドを `None` に保ったまま運用してください。詳細は [`src/py_export/bridge.rs`](src/py_export/bridge.rs) のコメントを参照。

## 設定ファイル例

`examples/common.toml` と `examples/experiment.toml` に注釈付きサンプルがあります。

## インストール

### Rust (Cargo)

```toml
[dependencies]
gaussian_job_shared = { git = "https://github.com/miyake-ken/gaussian-job-shared.git", branch = "main" }
```

`gaussian_job_shared` は `slurm_async_runner` を **公開依存** として持ちます。SAR が `pyo3` の `extension-module` フィーチャを有効化していない (`default-features = false`) ため、ライブラリ依存として組み込んでも libpython のリンクが要求されることはありません。

### Python (uv)

```toml
[project]
dependencies = [
  "gaussian_job_shared @ git+https://github.com/miyake-ken/gaussian-job-shared.git@main",
  # SAR の pyclasses が必要なので併せてインストール
  "slurm_async_runner @ git+https://github.com/kkiyama117/slurm-async-runner.git@main",
]
```

### Python (Pixi)

GAUSSIAN_repo ワークスペースの常用配信チャネル:

```toml
[pypi-dependencies]
gaussian_job_shared = { git = "https://github.com/miyake-ken/gaussian-job-shared.git", branch = "main" }
slurm_async_runner   = { git = "https://github.com/kkiyama117/slurm-async-runner.git", branch = "main" }
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

`extension-module` フィーチャはここでは **意図的に無効** です。各 maturin ビルド時に `[tool.maturin] features = ["pyo3/extension-module"]` を pyproject.toml 側で指定することで、`stub_gen` バイナリが libpython へリンクできるよう分離されています。
