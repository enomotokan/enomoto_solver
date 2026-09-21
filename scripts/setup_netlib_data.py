"""One-time environment setup: fetches and decompresses the Netlib LP test
problem set into a local cache directory (default: `<repo>/.netlib_cache`).

Run this once per environment (fresh container, CI image, dev machine) that
needs to run `python -m enomoto_solver.benchmark_highs`. After this step,
that benchmark reads only from the local cache and never itself talks to
netlib.org — see docs/netlib-data.md for why Netlib's data is fetched here
rather than committed to this repository.

Usage:
    python scripts/setup_netlib_data.py [--cache-dir PATH]

Requires network access to netlib.org and a C compiler (gcc/cc) in PATH,
used once to build Netlib's own `emps` decompressor.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from netlib_fetch import ensure_emps, ensure_mps, ensure_problem_list  # noqa: E402


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=Path(__file__).resolve().parents[1] / ".netlib_cache",
        help="where to cache the fetched/decompressed data (default: <repo>/.netlib_cache)",
    )
    args = parser.parse_args()
    args.cache_dir.mkdir(parents=True, exist_ok=True)

    print(f"fetching Netlib LP test data into {args.cache_dir} (network access required for this step only)")
    emps = ensure_emps(args.cache_dir, network=True)
    names = ensure_problem_list(args.cache_dir, network=True)
    print(f"found {len(names)} problems listed at netlib.org/lp/data/")

    ok: list[str] = []
    failed: list[str] = []
    for name in names:
        mps_path = ensure_mps(name, args.cache_dir, emps, network=True)
        if mps_path is None:
            failed.append(name)
            print(f"  {name:12s} FAILED (fetch/decompress error)")
        else:
            ok.append(name)

    print(f"\ncached {len(ok)}/{len(names)} problems under {args.cache_dir / 'mps'}")
    if failed:
        print(f"failed: {', '.join(failed)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
