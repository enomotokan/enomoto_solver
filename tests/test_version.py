"""`enomoto_solver.__version__` が Cargo.toml の版 (pyproject.toml の version もここから取る) と同じかを確かめる。"""

import re
from pathlib import Path

import enomoto_solver


def test_version_matches_cargo():
    cargo = (Path(__file__).resolve().parents[1] / "Cargo.toml").read_text()
    m = re.search(r'^\[package\][^\[]*?^version\s*=\s*"([^"]+)"', cargo, re.M | re.S)
    assert m is not None
    assert enomoto_solver.__version__ == m.group(1)
