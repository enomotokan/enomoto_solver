"""論文用ベンチマークの問題データを取得・展開する (1 回だけ、ネットワークが要る)。

集合とキャッシュの場所 (いずれも .gitignore 済み):

  netlib       Netlib の有限最適解をもつ 93 問   .netlib_cache/mps/<名前>.mps
  infeas       Netlib の実行不能問題 29 問       .netlib_infeas_cache/mps/<名前>.mps
  infeas_dual  その 29 問の双対 (非有界)          .netlib_infeas_dual_cache/mps/<名前>_dual.mps
  kennington   Kennington の 16 問                .kennington_cache/mps/<名前>.mps
  mittelmann   Mittelmann LPopt の公開問題        .mittelmann_cache/raw/ (圧縮のまま。計測時に 1 問ずつ展開)

infeas_dual は scripts/make_dual_unbounded.py と同じ構成 (目的関数を 0 にした問題の双対) だが、
cplex2 も含めて 29 問すべて作る (make_dual_unbounded.py は cplex2 を除外している。cplex2 は
「ほぼ実行可能」な問題で、双対は数値的には非有界にならず、HiGHS は最適値 0 と判定する。
計測スクリプトは cplex2_dual の期待状態を定めず、どの結論でも解けたとみなす)。

Netlib の問題は Netlib の emps 形式で圧縮されているので、展開に C コンパイラ (gcc/cc) が要る
(scripts/netlib_fetch.py の ensure_emps)。infeas_dual の生成には highspy が要る。

使い方 (リポジトリのルートで):
    python scripts/paper_bench/prepare_data.py [--sets netlib infeas infeas_dual kennington mittelmann]
"""
from __future__ import annotations

import argparse
import gzip
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from netlib_fetch import ensure_emps, ensure_mps, ensure_problem_list, fetch  # noqa: E402

INFEAS_URL = "https://www.netlib.org/lp/infeas/"
KENNINGTON_URL = "https://www.netlib.org/lp/data/kennington/"

# netlib.org/lp/infeas/ の実行不能問題 (29 問)。
INFEAS_PROBLEMS = [
    "bgdbg1", "bgetam", "bgindy", "bgprtr", "box1", "ceria3d", "chemcom", "cplex1", "cplex2",
    "ex72a", "ex73a", "forest6", "galenet", "gosh", "gran", "greenbea", "itest2", "itest6",
    "klein1", "klein2", "klein3", "mondou2", "pang", "pilot4i", "qual", "reactor", "refinery",
    "vol1", "woodinfe",
]
# netlib.org/lp/data/kennington/ の 16 問。
KENNINGTON_PROBLEMS = [
    "cre-a", "cre-b", "cre-c", "cre-d", "ken-07", "ken-11", "ken-13", "ken-18",
    "osa-07", "osa-14", "osa-30", "osa-60", "pds-02", "pds-06", "pds-10", "pds-20",
]

CACHES = {
    "netlib": REPO / ".netlib_cache",
    "infeas": REPO / ".netlib_infeas_cache",
    "infeas_dual": REPO / ".netlib_infeas_dual_cache",
    "kennington": REPO / ".kennington_cache",
    "mittelmann": REPO / ".mittelmann_cache",
}


def _emps_decompress(emps: Path, packed: Path, out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "wb") as f:
        proc = subprocess.run([str(emps), str(packed)], stdout=f, stderr=subprocess.PIPE)
    if proc.returncode != 0 or out.stat().st_size == 0:
        out.unlink(missing_ok=True)
        raise RuntimeError(f"emps failed on {packed}: {proc.stderr.decode(errors='replace')[-300:]}")


def _write_list(cache: Path, names: list[str]) -> None:
    (cache / "problems.txt").write_bytes(("\n".join(names) + "\n").encode())


def prepare_netlib() -> None:
    cache = CACHES["netlib"]
    cache.mkdir(parents=True, exist_ok=True)
    emps = ensure_emps(cache, network=True)
    names = ensure_problem_list(cache, network=True)
    bad = [n for n in names if ensure_mps(n, cache, emps, network=True) is None]
    print(f"netlib: {len(names) - len(bad)}/{len(names)} problems in {cache / 'mps'}")
    if bad:
        raise RuntimeError(f"netlib: failed to fetch {bad}")


def prepare_infeas() -> None:
    cache = CACHES["infeas"]
    cache.mkdir(parents=True, exist_ok=True)
    emps = ensure_emps(cache, network=True)
    for name in INFEAS_PROBLEMS:
        mps = cache / "mps" / f"{name}.mps"
        if mps.exists() and mps.stat().st_size > 0:
            continue
        raw = cache / "raw" / name
        raw.parent.mkdir(parents=True, exist_ok=True)
        if not raw.exists():
            fetch(INFEAS_URL + name, raw, timeout=120)
        _emps_decompress(emps, raw, mps)
    _write_list(cache, INFEAS_PROBLEMS)
    print(f"infeas: {len(INFEAS_PROBLEMS)} problems in {cache / 'mps'}")


def prepare_infeas_dual() -> None:
    from make_dual_unbounded import dual_of_feasibility_problem
    import highspy

    src, cache = CACHES["infeas"], CACHES["infeas_dual"]
    (cache / "mps").mkdir(parents=True, exist_ok=True)
    names = []
    for name in INFEAS_PROBLEMS:
        dname = f"{name}_dual"
        names.append(dname)
        out_path = cache / "mps" / f"{dname}.mps"
        if out_path.exists() and out_path.stat().st_size > 0:
            continue
        h = highspy.Highs()
        h.setOptionValue("output_flag", False)
        status = h.readModel(str(src / "mps" / f"{name}.mps"))
        if "kOk" not in str(status) and "kWarning" not in str(status):
            raise RuntimeError(f"{name}: read failed ({status})")
        dual = dual_of_feasibility_problem(h.getLp())
        out = highspy.Highs()
        out.setOptionValue("output_flag", False)
        out.passModel(dual)
        out.writeModel(str(out_path))
        print(f"  {dname}: {dual.num_row_} rows, {dual.num_col_} cols")
    _write_list(cache, names)
    print(f"infeas_dual: {len(names)} problems in {cache / 'mps'}")


def prepare_kennington() -> None:
    cache = CACHES["kennington"]
    cache.mkdir(parents=True, exist_ok=True)
    emps = ensure_emps(cache, network=True)
    for name in KENNINGTON_PROBLEMS:
        mps = cache / "mps" / f"{name}.mps"
        if mps.exists() and mps.stat().st_size > 0:
            continue
        raw = cache / "raw" / f"{name}.gz"
        raw.parent.mkdir(parents=True, exist_ok=True)
        if not raw.exists():
            fetch(KENNINGTON_URL + f"{name}.gz", raw, timeout=600)
        packed = cache / "raw" / name  # gzip を外した emps 形式
        packed.write_bytes(gzip.decompress(raw.read_bytes()))
        _emps_decompress(emps, packed, mps)
        packed.unlink()
    _write_list(cache, KENNINGTON_PROBLEMS)
    print(f"kennington: {len(KENNINGTON_PROBLEMS)} problems in {cache / 'mps'}")


def prepare_mittelmann() -> None:
    from run_mittelmann_benchmark import PROBLEMS, ensure_raw

    cache = CACHES["mittelmann"]
    cache.mkdir(parents=True, exist_ok=True)
    for name, rel, *_ in PROBLEMS:
        ensure_raw(cache, name, rel)
    print(f"mittelmann: {len(PROBLEMS)} problems (compressed) in {cache / 'raw'}")


STEPS = {
    "netlib": prepare_netlib,
    "infeas": prepare_infeas,
    "infeas_dual": prepare_infeas_dual,
    "kennington": prepare_kennington,
    "mittelmann": prepare_mittelmann,
}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sets", nargs="*", default=list(STEPS), choices=list(STEPS))
    args = ap.parse_args()
    for s in args.sets:
        if s == "infeas_dual" and "infeas" not in args.sets:
            prepare_infeas()
        STEPS[s]()


if __name__ == "__main__":
    main()
