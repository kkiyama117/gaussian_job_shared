#!/bin/bash
#============ Slurm Options ===========
#SBATCH --partition=gr10641a
#SBATCH --job-name=GAUSSIAN
#SBATCH --time=48:00:00
#SBATCH --output=/LARGE0/gr10641/.cache/slurm/logs/%x.%j.out
#SBATCH --error=/LARGE0/gr10641/.cache/slurm/logs/%x.%j.err
#SBATCH --rsc p=1:t=16:c=16:m=16G
#SBATCH --mail-user=k.kiyama117@gmail.com
#SBATCH --mail-type=BEGIN,END,FAIL
#============ Shell Script ============

# ----------------------------------------------------------------------------
# INITIALIZE THE ENVIRONMENT FOR SLURM
# ----------------------------------------------------------------------------
# Set `e` and `u` flag and pipefail
set -euo pipefail

# Wipe any conda activation state inherited from the parent (login)
# shell — both env vars and the `conda` shell function. SLURM jobs
# inherit whatever conda activation the login shell had, and the
# `module restore` below triggers module unload hooks that internally
# call `conda deactivate`. If the inherited CONDA_PREFIX_<N> stack is
# partially corrupt (which happens when the parent shell ever ran
# `pixi shell-hook` — pixi overwrites CONDA_PREFIX without bumping
# CONDA_SHLVL), that deactivate aborts with "non-consecutive
# CONDA_PREFIX_<number>". Resetting state BEFORE anything that might
# call conda guarantees a clean slate.
set +u
unset -f conda 2>/dev/null || true
for _v in $(env 2>/dev/null | awk -F= '/^CONDA_/{print $1}'); do
    unset "$_v" || true
done
unset _v
set -u

# Load `conda` , for `module` can call `unload hook` with `conda deactivate`
set +u
source "$(conda info --base)/etc/profile.d/conda.sh"
set -u

# Reload shell config for slurm
. /usr/share/Modules/init/bash

# Load module(s)
module restore gaussian_A -f

# Activate the conda env that ships `pixi` so the `pixi run` calls in the
# job body resolve. Workspace CLIs (gaussian-parse-results,
# python -m gaussian_compute_runtime ...) are NOT placed on PATH by this
# step; each invocation is wrapped in `pixi run [--manifest-path ...]` so
# the pixi-managed env activates per command, regardless of inherited PATH.
set +u
conda activate analysis
set -u

# ----------------------------------------------------------------------------
# JOB BODY
# ----------------------------------------------------------------------------

pixi run python -m gaussian_compute_runtime run-g16 --config "/LARGE0/gr10641/calc_data/GAUSSIAN/test_library/common.toml" --uuid "019dfd38-12bd-7000-88c5-eb12e20d765e"
echo "JOB DONE"
exit 0
