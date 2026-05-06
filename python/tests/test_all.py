import gaussian_job_shared
from gaussian_job_shared._core import sum_as_string


def test_sum_as_string():
    assert sum_as_string(1, 1) == "2"
