"""Regression check for smoke.py's inherited Git environment isolation."""
import importlib.util
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("smoke", "scripts/smoke.py")
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)
with mock.patch.dict(os.environ, {"gIt_TEST_SENTINEL": "inherited", "SMOKE_SENTINEL": "kept"}):
    with mock.patch.object(smoke, "git", side_effect=RuntimeError("capture")) as git:
        try:
            smoke.main(sys.argv[1])
        except RuntimeError as error:
            assert str(error) == "capture"
        env = git.call_args.args[1]
        assert not any(key.upper() == "GIT_TEST_SENTINEL" for key in env)
        assert env["SMOKE_SENTINEL"] == "kept"

with tempfile.TemporaryDirectory(prefix="yadot-smoke-env-") as directory:
    root = Path(directory)
    index = root / "index"
    objects = root / "objects"
    objects.mkdir()
    env = os.environ.copy()
    env.update({
        "GIT_INDEX_FILE": str(index),
        "GIT_OBJECT_DIRECTORY": str(objects),
        "GIT_CONFIG_COUNT": "1",
        "GIT_CONFIG_KEY_0": "init.defaultBranch",
        "GIT_CONFIG_VALUE_0": "wrong-branch",
        "gIt_TEST_SENTINEL": "inherited",
    })
    result = subprocess.run([sys.executable, "scripts/smoke.py", sys.argv[1]], env=env)
    assert result.returncode == 0
    assert not index.exists()
    assert not any(objects.iterdir())
