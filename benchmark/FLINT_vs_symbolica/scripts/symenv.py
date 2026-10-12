"""Environment for Symbolica subprocesses: single threaded, plus the variables
exported by the license file (SYMGCAD_ENV, default <repo>/.symgcad-env).
Without a license Symbolica runs restricted (one thread per user), which is
enough for sequential conversion but not for two concurrent timing workers.
Values are passed through, never printed."""
import os, shlex

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))


def symbolica_env():
    env = dict(os.environ, RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1")
    path = os.environ.get("SYMGCAD_ENV", os.path.join(REPO, ".symgcad-env"))
    if os.path.isfile(path):
        for line in open(path):
            line = line.strip()
            if line.startswith("export "):
                line = line[len("export "):]
            if "=" not in line or line.startswith("#"):
                continue
            name, value = line.split("=", 1)
            parts = shlex.split(value)
            env[name.strip()] = parts[0] if parts else ""
    return env
