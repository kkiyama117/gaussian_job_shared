"""Smoke tests for the PyO3-exported Rust entities.

Covers: legacy `sum_as_string`, every newtype, every enum, every stringly
encoded surface type, and a small `JobFlow` that mirrors the g16 → post pair
from `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`.
"""

from datetime import datetime, timezone

import pytest

from gaussian_job_shared._core import sum_as_string
from slurm_async_runner._slurm_async_runner_core.entities.slurm.sbatch_options import (
    ArrayIndex,
    DependencyClause,
    DependencyJobRef,
    DependencyJoin,
    DependencyType,
    JobTimeLimit,
    MailType,
    MailTypeInput,
    Memory,
    MemoryUnit,
    ResourceSpec,
    ResourceSpecGPU,
    SlurmArraySpec,
    SlurmDependency,
    SlurmJobConfig,
)
from slurm_async_runner._slurm_async_runner_core.entities.slurm.status import (
    JobReason,
    JobState,
    JobStatus,
)
from gaussian_job_shared._core.entities.workflow import (
    CalcType,
    Job,
    JobEdge,
    JobFlow,
    JobId,
    JobSpec,
    Program,
)


# --------------------------------------------------------------------- legacy
def test_sum_as_string():
    assert sum_as_string(1, 1) == "2"


def test_module_all_includes_new_types():
    from gaussian_job_shared._core import entities as shared_entities
    from slurm_async_runner._slurm_async_runner_core import entities as sar_entities

    assert "JobFlow" in shared_entities.workflow.__all__
    assert "JobStatus" in sar_entities.slurm.status.__all__
    assert "DependencyType" in sar_entities.slurm.sbatch_options.__all__


# ------------------------------------------------------------------- newtypes
def test_jobid_roundtrip_and_equality():
    a = JobId("g16")
    b = JobId("g16")
    c = JobId("post")
    assert str(a) == "g16"
    assert a.value == "g16"
    assert a == b
    assert a != c
    assert hash(a) == hash(b)
    assert a < c  # alphabetical Ord
    assert repr(a) == 'JobId("g16")'


def test_program_and_calctype_basic():
    assert Program("g16").value == "g16"
    assert str(CalcType("opt")) == "opt"
    assert CalcType("a") < CalcType("b")


# ----------------------------------------------------------------------- enums
def test_dependency_type_variants_and_str():
    assert str(DependencyType.AfterOk) == "afterok"
    assert str(DependencyType.AfterAny) == "afterany"
    assert str(DependencyType.Singleton) == "singleton"
    assert DependencyType.AfterOk == DependencyType.AfterOk
    assert DependencyType.AfterOk != DependencyType.AfterAny


def test_job_state_str_and_token():
    assert str(JobState.Pending) == "PENDING"
    assert str(JobState.OutOfMemory) == "OUT_OF_MEMORY"
    assert JobState.Completed.token == "COMPLETED"
    assert JobState.Unknown.token == "UNKNOWN"


def test_job_state_parse():
    # Long form
    assert JobState.parse("PENDING") == JobState.Pending
    assert JobState.parse("RUNNING") == JobState.Running
    assert JobState.parse("COMPLETED") == JobState.Completed
    assert JobState.parse("OUT_OF_MEMORY") == JobState.OutOfMemory

    # Compact code
    assert JobState.parse("PD") == JobState.Pending
    assert JobState.parse("OOM") == JobState.OutOfMemory
    assert JobState.parse("CD") == JobState.Completed
    assert JobState.parse("NF") == JobState.NodeFail

    # Trailing context + case-insensitive
    assert JobState.parse("CANCELLED by 1234") == JobState.Cancelled
    assert JobState.parse("  RUNNING  ") == JobState.Running
    assert JobState.parse("pending") == JobState.Pending

    # Unknown / empty / legacy lowercase tokens are no longer mapped
    assert JobState.parse("") == JobState.Unknown
    assert JobState.parse("FOO_BAR") == JobState.Unknown
    assert JobState.parse("queued") == JobState.Unknown
    assert JobState.parse("done") == JobState.Unknown


def test_job_reason_constructors():
    assert JobReason.none().name == "None"
    assert JobReason.priority().name == "Priority"
    assert JobReason.resources().name == "Resources"
    assert JobReason.dependency().name == "Dependency"
    assert JobReason.begin_time().name == "BeginTime"
    assert JobReason.time_limit().name == "TimeLimit"
    assert JobReason.out_of_memory().name == "OutOfMemory"
    assert JobReason.non_zero_exit_code().name == "NonZeroExitCode"


def test_job_reason_parse_known():
    assert JobReason.parse("None") == JobReason.none()
    assert JobReason.parse("Priority") == JobReason.priority()
    assert JobReason.parse("ReqNodeNotAvail").name == "NodeNotAvail"
    assert JobReason.parse("AssocGrpCpuLimit").name == "AssocGrpCpuLimit"


def test_job_reason_parse_empty_is_none():
    assert JobReason.parse("") == JobReason.none()
    assert JobReason.parse("   ") == JobReason.none()


def test_job_reason_parse_unknown_falls_back_to_other():
    r = JobReason.parse("FuturisticReason")
    assert r.name == "Other"
    assert r.value == "FuturisticReason"
    assert str(r) == "FuturisticReason"


def test_job_reason_other_constructor_is_explicit():
    # Even a known canonical name stored verbatim via .other()
    r = JobReason.other("Priority")
    assert r.name == "Other"
    assert r.value == "Priority"


def test_job_reason_value_round_trips_via_parse():
    for r in [
        JobReason.none(),
        JobReason.priority(),
        JobReason.resources(),
        JobReason.parse("AssocMaxJobsLimit"),
        JobReason.other("CustomReason"),
    ]:
        assert JobReason.parse(r.value) == r if r.name != "Other" else True


def test_job_status_default_and_constructors():
    s = JobStatus(JobState.Pending)
    assert s.state == JobState.Pending
    assert s.reason == JobReason.none()

    s2 = JobStatus(JobState.Pending, JobReason.priority())
    assert s2.state == JobState.Pending
    assert s2.reason == JobReason.priority()


def test_job_status_parse_state_and_reason():
    s = JobStatus.parse("PD", "Priority")
    assert s.state == JobState.Pending
    assert s.reason == JobReason.priority()

    # Reason omitted defaults to None
    s2 = JobStatus.parse("R")
    assert s2.state == JobState.Running
    assert s2.reason == JobReason.none()


def test_job_status_equality_and_hash():
    a = JobStatus(JobState.Pending, JobReason.priority())
    b = JobStatus(JobState.Pending, JobReason.priority())
    c = JobStatus(JobState.Pending, JobReason.resources())
    assert a == b
    assert a != c
    assert hash(a) == hash(b)


def test_job_status_str():
    s = JobStatus(JobState.Pending, JobReason.priority())
    assert str(s) == "PENDING (Priority)"

    s2 = JobStatus(JobState.Running)
    assert str(s2) == "RUNNING (None)"


def test_mail_type_str():
    assert str(MailType.BEGIN) == "BEGIN"
    assert str(MailType.ALL) == "ALL"


def test_memory_unit_str():
    assert str(MemoryUnit.Giga) == "G"
    assert str(MemoryUnit.Mega) == "M"


def test_dependency_join_str():
    assert str(DependencyJoin.And) == ","
    assert str(DependencyJoin.Or) == "?"


# ---------------------------------------------------- stringly-encoded types
def test_job_time_limit_parse_and_str():
    t = JobTimeLimit("3-12:00:00")
    assert t.total_seconds == 3 * 86_400 + 12 * 3600
    assert str(t) == "84:00:00"
    assert JobTimeLimit.from_seconds(3600).total_seconds == 3600


def test_job_time_limit_rejects_zero():
    with pytest.raises(ValueError):
        JobTimeLimit("0")


def test_memory_parse_unit_default_and_explicit():
    assert Memory("8G").value == 8
    assert Memory("8G").unit == MemoryUnit.Giga
    assert Memory("1024").unit == MemoryUnit.Mega
    assert str(Memory.from_value(4, MemoryUnit.Kilo)) == "4K"


def test_resource_spec_cpu_round_trip():
    r = ResourceSpec.from_str("p=4:t=8:c=8:m=8G")
    assert r.kind == "cpu"
    assert r.cpu_spec is not None
    assert r.cpu_spec.p == 4
    assert r.cpu_spec.m.value == 8
    assert r.cpu_spec.m.unit == MemoryUnit.Giga
    assert str(r) == "p=4:t=8:c=8:m=8G"


def test_resource_spec_gpu_round_trip():
    r = ResourceSpec.gpu(ResourceSpecGPU(2))
    assert r.kind == "gpu"
    assert r.gpu_spec is not None
    assert r.gpu_spec.g == 2
    assert str(r) == "g=2"


def test_resource_spec_rejects_zero_cpu():
    with pytest.raises(ValueError):
        ResourceSpec.from_str("p=0:t=1:c=1:m=1G")


def test_slurm_array_spec_parse():
    s = SlurmArraySpec("0-15:4%2")
    assert str(s) == "0-15:4%2"
    assert s.max_concurrent == 2
    assert len(s.indices) == 1
    assert s.indices[0].kind == "stepped"
    assert s.indices[0].step == 4


def test_array_index_constructors():
    assert ArrayIndex.single(5).kind == "single"
    assert ArrayIndex.single(5).value == 5
    assert ArrayIndex.range(0, 9).kind == "range"
    assert ArrayIndex.range(0, 9).start == 0
    assert ArrayIndex.range(0, 9).end == 9
    assert ArrayIndex.stepped(0, 15, 4).step == 4


def test_slurm_dependency_parse_and_clauses():
    d = SlurmDependency("afterok:200,afterany:201")
    assert str(d) == "afterok:200,afterany:201"
    assert d.join == DependencyJoin.And
    assert len(d.clauses) == 2
    assert d.clauses[0].dep_type == DependencyType.AfterOk
    assert d.clauses[0].job_refs[0].job_id == 200


def test_slurm_dependency_from_clauses():
    d = SlurmDependency.from_clauses(
        [DependencyClause(DependencyType.AfterOk, [DependencyJobRef(99)])],
        DependencyJoin.And,
    )
    assert str(d) == "afterok:99"


def test_dependency_job_ref_with_delay():
    r = DependencyJobRef(200, 5)
    assert str(r) == "200+5"


# ---------------------------------------------------------------- compounds
def _bare_config(partition: str = "long") -> SlurmJobConfig:
    return SlurmJobConfig(partition=partition)


def test_slurm_job_config_construction_and_setters():
    cfg = SlurmJobConfig(
        partition="long",
        time_limit=JobTimeLimit("01:00:00"),
        comment="hello",
        resource_spec=ResourceSpec.from_str("p=1:t=1:c=1:m=1G"),
    )
    assert cfg.partition == "long"
    assert cfg.time_limit.total_seconds == 3600
    cfg.partition = "gr10001b"
    assert cfg.partition == "gr10001b"
    cfg.time_limit = None
    assert cfg.time_limit is None


def test_mail_type_input_from_list_and_parse():
    inp = MailTypeInput([MailType.BEGIN, MailType.END])
    assert inp == MailTypeInput.parse("BEGIN,END")
    with pytest.raises(ValueError):
        MailTypeInput([])


def test_job_edge_field_access_with_reserved_word_alias():
    e = JobEdge(JobId("g16"), DependencyType.AfterOk)
    assert e.from_ == JobId("g16")
    assert e.kind == DependencyType.AfterOk
    e.kind = DependencyType.AfterAny
    assert e.kind == DependencyType.AfterAny


def test_job_spec_and_job_basic():
    spec = JobSpec(
        program=Program("g16"),
        config=_bare_config(),
        body="g16 < input.gjf > output.log\n",
    )
    job = Job(spec=spec, parents=[])
    assert job.spec.program == Program("g16")
    assert job.parents == []
    job.parents = [JobEdge(JobId("upstream"), DependencyType.AfterOk)]
    assert len(job.parents) == 1
    assert job.parents[0].from_ == JobId("upstream")


# ------------------------------------------------------------------- JobFlow
def test_job_flow_g16_post_pair():
    """Mirrors §1.1 of the slurm-job-flow-structs spec."""
    flow = JobFlow(
        uuid=JobFlow.new_uuid(),
        created_at=datetime(2026, 5, 8, tzinfo=timezone.utc),
        work_dir="/tmp/flow-pyffi",
    )
    flow.insert_job(
        JobId("g16"),
        Job(JobSpec(Program("g16"), _bare_config(), ""), []),
    )
    flow.insert_job(
        JobId("post"),
        Job(
            JobSpec(Program("formchk"), _bare_config(), ""),
            [JobEdge(JobId("g16"), DependencyType.AfterOk)],
        ),
    )
    assert sorted(flow.jobs.keys()) == ["g16", "post"]
    post = flow.get_job(JobId("post"))
    assert post is not None
    assert len(post.parents) == 1
    assert post.parents[0].from_ == JobId("g16")
    # tags is an empty dict by default
    assert flow.tags == {}
    # work_dir round-trips as Path
    assert str(flow.work_dir) == "/tmp/flow-pyffi"


def test_job_flow_uuid_setter_validates():
    flow = JobFlow(
        uuid=JobFlow.new_uuid(),
        created_at=datetime(2026, 5, 8, tzinfo=timezone.utc),
        work_dir="/tmp/flow",
    )
    with pytest.raises(ValueError):
        flow.uuid = "not-a-uuid"
