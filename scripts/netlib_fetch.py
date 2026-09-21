"""Fetch-and-cache logic for the Netlib LP test problem set.

Shared by `scripts/setup_netlib_data.py` (the one-time, network-using setup
step) and `python/enomoto_solver/benchmark_highs.py` (which only ever reads
from the cache that setup step populates — see docs/netlib-data.md for why
Netlib's data itself is never committed to this repository).

Deliberately dependency-free (stdlib only) so the setup step doesn't require
this crate's compiled extension or `highspy` to already be installed.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

NETLIB_INDEX_URL = "https://www.netlib.org/lp/data/"
EMPS_C_URL = "https://www.netlib.org/lp/data/emps.c"

# Netlib "problems" that aren't plain compressed-MPS files: `minos` is a
# plain-text readme, `stocfor3`/`truss` are Fortran-source-plus-data
# bundles that need their own generator program, not `emps`. Excluded
# outright rather than attempted and reported as failures every run.
NON_MPS_ENTRIES = {"minos", "stocfor3", "truss", "ascii", "changes", "readme"}

# The rest of the index's non-problem entries are recognizable by suffix:
# the decompressor's own sources (`emps.c`, `emps.f`), archives
# (`emps.exe.gz`, `nams.ps.gz`), `mpc.src`, and superseded copies
# (`stocfor3.old`). Matched by suffix rather than listed by name so a new
# file of the same kind doesn't silently become a "problem" that fails to
# decompress. Real problem names that merely look extension-like
# (`pilot.ja`, `pilot.we`, `vtp.base`) are unaffected.
NON_PROBLEM_SUFFIXES = (".c", ".f", ".gz", ".z", ".src", ".old", ".html", ".ps", ".pdf", ".txt")


class NetlibCacheError(RuntimeError):
    """Needed Netlib data isn't cached and network access is disabled for
    this call — the caller should point the user at
    `scripts/setup_netlib_data.py`."""


def fetch(url: str, dest: Path, timeout: int = 30) -> None:
    with urllib.request.urlopen(url, timeout=timeout) as resp:
        dest.write_bytes(resp.read())


def ensure_emps(cache_dir: Path, *, network: bool) -> Path:
    """Returns the path to the compiled Netlib `emps` decompressor, cached
    under `cache_dir`. The first call requires `network=True` (and a C
    compiler in PATH, e.g. gcc, to build it)."""
    exe = cache_dir / ("emps.exe" if sys.platform == "win32" else "emps")
    if exe.exists():
        return exe
    if not network:
        raise NetlibCacheError(
            f"{exe} not found — run `python scripts/setup_netlib_data.py` once to fetch and build it"
        )
    cc = shutil.which("gcc") or shutil.which("cc")
    if cc is None:
        raise RuntimeError("no C compiler (gcc/cc) found in PATH — required once to build Netlib's `emps` decompressor")
    src = cache_dir / "emps.c"
    if not src.exists():
        fetch(EMPS_C_URL, src)
    subprocess.run([cc, "-O2", "-o", str(exe), str(src)], check=True, capture_output=True)
    return exe


def ensure_problem_list(cache_dir: Path, *, network: bool) -> list[str]:
    """Returns the cached list of Netlib LP problem names, fetching
    `netlib.org/lp/data/`'s index the first time (`network=True` required
    then)."""
    list_path = cache_dir / "problems.txt"
    if list_path.exists():
        return [line.strip() for line in list_path.read_text().splitlines() if line.strip()]
    if not network:
        raise NetlibCacheError(f"{list_path} not found — run `python scripts/setup_netlib_data.py` once to fetch it")
    index = cache_dir / "index.html"
    fetch(NETLIB_INDEX_URL, index)
    # Problem names are not restricted to `[a-z0-9_]`: Netlib's own set
    # includes `gfrd-pnc`, `maros-r7`, `pilot.ja`, `pilot.we` and
    # `vtp.base`, so dots and hyphens have to be accepted here and the
    # non-problem entries filtered out explicitly instead (links into
    # subdirectories such as `kennington/` are skipped outright).
    hrefs = sorted(set(re.findall(r'<a href="([^"]+)">', index.read_text(errors="replace"))))
    names = [
        n
        for n in hrefs
        if "/" not in n and not n.lower().endswith(NON_PROBLEM_SUFFIXES) and n not in NON_MPS_ENTRIES
    ]
    list_path.write_text("\n".join(names))
    return names


def ensure_mps(name: str, cache_dir: Path, emps: Path, *, network: bool) -> Path | None:
    """Returns the path to `name`'s decompressed `.mps` file, cached under
    `cache_dir`. Returns `None` if it isn't cached and `network=False`, or
    if decompression fails (when fetching with `network=True`)."""
    mps_path = cache_dir / "mps" / f"{name}.mps"
    if mps_path.exists() and mps_path.stat().st_size > 0:
        return mps_path
    if not network:
        return None
    raw_path = cache_dir / "raw" / name
    raw_path.parent.mkdir(parents=True, exist_ok=True)
    if not raw_path.exists():
        fetch(NETLIB_INDEX_URL + name, raw_path)
    mps_path.parent.mkdir(parents=True, exist_ok=True)
    with open(mps_path, "wb") as out:
        proc = subprocess.run([str(emps), str(raw_path)], stdout=out, stderr=subprocess.PIPE)
    if proc.returncode != 0 or mps_path.stat().st_size == 0:
        mps_path.unlink(missing_ok=True)
        return None
    return mps_path
