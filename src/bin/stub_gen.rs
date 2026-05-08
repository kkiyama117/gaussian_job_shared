//! pyo3-stub-gen entry point. Re-exposes `gaussian_job_shared::stub_info`
//! (defined in `src/py_export/mod.rs` via `define_stub_info_gatherer!`).
//! Only built when the `stub_gen` feature is on.

fn main() -> pyo3_stub_gen::Result<()> {
    let stub = gaussian_job_shared::stub_info()?;
    stub.generate()?;
    Ok(())
}
