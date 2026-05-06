import pytest
import gaussian_job_shared


def test_sum_as_string():
    assert gaussian_job_shared.sum_as_string(1, 1) == "2"
