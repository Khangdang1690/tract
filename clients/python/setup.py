"""setuptools entrypoint.

In normal builds (M5 packaging path) we run cargo and bundle the daemon
binary into ``tract/bin/``. For local development the binary is found via
the ``TRACTD_BINARY`` env var or PATH, so the build step is opt-out via
``TRACT_SKIP_CARGO=1``.
"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
from pathlib import Path

from setuptools import setup
from setuptools.command.build_py import build_py
from setuptools.dist import Distribution

REPO_ROOT = Path(__file__).resolve().parents[2]
PKG_DIR = Path(__file__).resolve().parent / "tract"
BIN_DIR = PKG_DIR / "bin"
BIN_NAME = "tractd.exe" if platform.system() == "Windows" else "tractd"


class BuildPy(build_py):
    def run(self) -> None:
        if os.environ.get("TRACT_SKIP_CARGO") != "1":
            self._cargo_build()
        super().run()

    def _cargo_build(self) -> None:
        cargo = os.environ.get("CARGO", "cargo")
        subprocess.check_call(
            [cargo, "build", "--release", "-p", "tractd"],
            cwd=str(REPO_ROOT),
        )
        BIN_DIR.mkdir(parents=True, exist_ok=True)
        src = REPO_ROOT / "target" / "release" / BIN_NAME
        if not src.exists():
            raise FileNotFoundError(f"daemon binary not found at {src}")
        shutil.copy2(src, BIN_DIR / BIN_NAME)


class BinaryDistribution(Distribution):
    """Tells setuptools this wheel is platform-specific (it bundles a daemon binary)."""

    def has_ext_modules(self) -> bool:
        return True


setup(cmdclass={"build_py": BuildPy}, distclass=BinaryDistribution)
