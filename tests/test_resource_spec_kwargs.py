"""Verify the positional/kwargs constructor and from_str classmethod
on gaussian_job_shared._core.entities.slurm.sbatch_options.ResourceSpec.

These tests run inside the maturin develop build, i.e. against the
.so produced by `maturin develop` in this repo.
"""

import pytest

from gaussian_job_shared._core.entities.slurm.sbatch_options import (
    Memory,
    ResourceSpec,
)


def test_resource_spec_full_cpu_via_positional_args():
    r = ResourceSpec(4, 8, 8, Memory("2G"))
    assert str(r) == "p=4:t=8:c=8:m=2G"


def test_resource_spec_full_cpu_via_kwargs():
    r = ResourceSpec(processes=4, threads=8, cores=8, memory=Memory("2G"))
    assert str(r) == "p=4:t=8:c=8:m=2G"


def test_resource_spec_partial_cpu():
    r = ResourceSpec(processes=60, threads=1, cores=1)
    assert str(r) == "p=60:t=1:c=1"


def test_resource_spec_memory_only():
    r = ResourceSpec(memory=Memory("8G"))
    assert str(r) == "m=8G"


def test_resource_spec_default_constructor_renders_empty_cpu():
    r = ResourceSpec()
    assert str(r) == ""
    assert r.kind == "cpu"


def test_resource_spec_gpu():
    r = ResourceSpec(gpus=1)
    assert str(r) == "g=1"
    assert r.kind == "gpu"


def test_resource_spec_rejects_mixed_cpu_and_gpu():
    with pytest.raises(ValueError, match="mutually exclusive"):
        ResourceSpec(processes=4, gpus=1)


def test_resource_spec_rejects_zero_count():
    with pytest.raises(ValueError, match="must be > 0"):
        ResourceSpec(gpus=0)
    with pytest.raises(ValueError, match="must be > 0"):
        ResourceSpec(processes=0, threads=1, cores=1)


def test_resource_spec_memory_must_be_pymemory():
    # Strict typing — string is not implicitly converted.
    with pytest.raises(TypeError):
        ResourceSpec(processes=4, threads=8, cores=8, memory="2G")


def test_resource_spec_from_str_classmethod():
    r = ResourceSpec.from_str("p=4:t=8:c=8:m=8G")
    assert str(r) == "p=4:t=8:c=8:m=8G"

    r2 = ResourceSpec.from_str("g=1")
    assert str(r2) == "g=1"


def test_resource_spec_from_str_rejects_empty():
    with pytest.raises(ValueError):
        ResourceSpec.from_str("")
