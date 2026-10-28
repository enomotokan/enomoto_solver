"""試作の比べ: 問題 x 設定 x 回を、run_miplib_benchmark.py --worker-ours (4 スレッド・60 秒) で 1 つずつ解き、JSON Lines に書き足す。

使い方: python bench.py <out.jsonl> --configs base subrel ... --problems a b ... [--runs 2]
        python bench.py <out.jsonl> --summary [--highs <run_miplib_benchmark の JSON> ...]

設定は CONFIGS (環境変数の組)。回ごとに問題を回し、問題ごとに設定を順に解く (時間の揺れが設定に偏らないように)。
同じ (設定, 問題, 回) が out にあれば飛ばす (中断しても再開できる)。
"""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
import run_miplib_benchmark as rmb  # noqa: E402

CONFIGS = {
    "base": {},
    "fd": {"ENOMOTO_MIP_SUBMIP_FIRST_DONE": "1"},
    "subrel": {"ENOMOTO_MIP_ROOT_SUBMIP_REL": "1", "ENOMOTO_MIP_SUBMIP_FIRST_DONE": "1"},
    "subrel2": {"ENOMOTO_MIP_ROOT_SUBMIP_REL": "2", "ENOMOTO_MIP_SUBMIP_FIRST_DONE": "1"},
    "epr": {"ENOMOTO_MIP_PROBE_EARLY_REPRESOLVE": "1"},
    "rcf": {"ENOMOTO_MIP_RESTART_CUTS_FIXED": "1"},
    "eprrcf": {"ENOMOTO_MIP_PROBE_EARLY_REPRESOLVE": "1", "ENOMOTO_MIP_RESTART_CUTS_FIXED": "1"},
    "ostop": {"ENOMOTO_MIP_SUBMIP_OUTCOME_STOP": "1"},
    "eprostop": {"ENOMOTO_MIP_PROBE_EARLY_REPRESOLVE": "1", "ENOMOTO_MIP_SUBMIP_OUTCOME_STOP": "1"},
    "all": {"ENOMOTO_MIP_PROBE_EARLY_REPRESOLVE": "1", "ENOMOTO_MIP_RESTART_CUTS_FIXED": "1",
            "ENOMOTO_MIP_ROOT_SUBMIP_REL": "1", "ENOMOTO_MIP_SUBMIP_FIRST_DONE": "1"},
}


def run_one(cfg: str, name: str, threads: int, tl: float) -> dict:
    mps = rmb.fetch(name, REPO / ".miplib_cache")
    env = dict(os.environ, ENOMOTO_MIP_THREADS=str(threads), **CONFIGS[cfg])
    cmd = [sys.executable, str(REPO / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(threads)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=tl * 1.5 + 60, env=env)
    except subprocess.TimeoutExpired:
        return {"status": "killed", "objective": None, "time": None}
    for line in reversed(p.stdout.splitlines()):
        if line.startswith("{"):
            r = json.loads(line)
            r["warnings"] = p.stderr.count("MIP: WARNING")
            return r
    return {"status": f"crash rc={p.returncode}", "objective": None, "time": None, "stderr": p.stderr[-1000:]}


def sgm(ts, shift=10.0):
    return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift if ts else float("nan")


def summary(out: Path, tl: float, highs: list[Path]) -> None:
    rows = [json.loads(l) for l in out.read_text().splitlines() if l.strip()]
    ref = {}
    for h in highs:
        for n, r in json.loads(h.read_text()).items():
            if r["highs"].get("status") == "Optimal":
                ref.setdefault(n, r["highs"]["objective"])
    cfgs = list(dict.fromkeys(r["cfg"] for r in rows))
    names = list(dict.fromkeys(r["name"] for r in rows))
    runs = sorted({r["run"] for r in rows})
    by = {(r["cfg"], r["name"], r["run"]): r for r in rows}
    print("| 設定 | 回 | 解けた数 | SGM [s] | 不一致 |")
    print("|---|---:|---:|---:|---|")
    for c in cfgs:
        for k in runs:
            rs = [by.get((c, n, k)) for n in names]
            if any(r is None for r in rs):
                continue
            ts = [r["time"] if rmb.solved(r) else tl for r in rs]
            ns = sum(rmb.solved(r) for r in rs)
            bad = [n for n, r in zip(names, rs) if rmb.solved(r) and n in ref and abs(r["objective"] - ref[n]) > 1e-4 * max(1.0, abs(ref[n])) + 1e-6]
            print(f"| {c} | {k} | {ns}/{len(names)} | {sgm(ts):.2f} | {len(bad)} {bad if bad else ''} |")
    print()
    print("| 問題 | " + " | ".join(cfgs) + " |")
    print("|---|" + "---:|" * len(cfgs))
    for n in names:
        cells = []
        for c in cfgs:
            v = []
            for k in runs:
                r = by.get((c, n, k))
                if r is None:
                    continue
                v.append(f"{r['time']:.1f}" if rmb.solved(r) else "t")
            cells.append(" / ".join(v))
        print(f"| {n} | " + " | ".join(cells) + " |")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("out", type=Path)
    ap.add_argument("--configs", nargs="*", default=["base"])
    ap.add_argument("--problems", nargs="*", default=None)
    ap.add_argument("--runs", type=int, default=2)
    ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--time-limit", type=float, default=60.0)
    ap.add_argument("--summary", action="store_true")
    ap.add_argument("--highs", nargs="*", type=Path, default=[])
    a = ap.parse_args()
    if a.summary:
        return summary(a.out, a.time_limit, a.highs)
    names = a.problems or rmb.PROBLEMS
    done = set()
    if a.out.exists():
        for l in a.out.read_text().splitlines():
            if l.strip():
                r = json.loads(l)
                done.add((r["cfg"], r["name"], r["run"]))
    for k in range(1, a.runs + 1):
        for n in names:
            for c in a.configs:
                if (c, n, k) in done:
                    continue
                r = run_one(c, n, a.threads, a.time_limit)
                r.update(cfg=c, name=n, run=k)
                with open(a.out, "a") as f:
                    f.write(json.dumps(r) + "\n")
                print(f"{k} {n:24s} {c:8s} {r['status']:>10s} {r.get('time') or 0:7.2f} obj={r.get('objective')}", flush=True)


if __name__ == "__main__":
    main()
