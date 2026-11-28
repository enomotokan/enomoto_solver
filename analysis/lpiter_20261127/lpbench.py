"""LP の物差しで LPITER の試作を比べる (10 節、親の依頼: 純粋な線形代数の速さは LP で比べる)。

python analysis/lpiter_20261127/lpbench.py 集合 [--shard i/n] [--mode simplex|auto4] [--cfgs def,rtib,rtibo,highs] [--tag 名前]
  集合: netlib (有限の最適解あり 93 問) | kennington (16 問) | mittelmann (`MITTEL` の問題、.mittelmann_cache に取ってあるもの) | 問題名をカンマで並べたもの
  → lpbench_<tag>.jsonl (既定の tag は 集合_モード[_si]) に 1 回の求解ごとに追記する。止めても同じコマンドで続きから回る。

- 1 回の求解ごとに新しいプロセス。時間は Model.solve() / Highs.run() の前後だけ (MPS の読み込み・組み立ては含めない)。
- mode simplex: 1 スレッド (RAYON_NUM_THREADS=1、threads=1)、lp_solver="simplex"、上限 600 秒。3 回解いて中央値 (1 回目が 300 秒以上か
  解けなかったら 1 回だけ)。回ごとに設定の順を回す。HiGHS は simplex・1 スレッド。
- mode auto4: 既定の lp_solver="auto"・4 スレッド (二段解法と内点法の競争) で 1 回。HiGHS は既定 (choose)・4 スレッド。
- 当方は ENOMOTO_PROF_PHASES_EXT=1 で解き、標準エラーに出る段階別の時間 (単体法の呼び出しごとの表) を足し合わせて記録する
  (反復数 `iters`、`ftran`・`btran(rho_p)` などの ms)。計時の手間は 1 反復に十数回の時刻の読み取りで、どの設定にも同じだけかかる。
- 設定: def = 既定、rtib = RETA_TICK=1・LU_TINY_HS=1i・BTRAN_TINY=1、rtibo = rtib + BTRAN_OUT=1。
"""
import json, os, re, statistics, subprocess, sys, time
from pathlib import Path

D = Path(__file__).resolve().parent
REPO = D.parents[1]
TL = 600.0
RTIB = {"ENOMOTO_LU_RETA_TICK": "1", "ENOMOTO_LU_TINY_HS": "1i", "ENOMOTO_LU_BTRAN_TINY": "1"}
CFGS = {"def": {}, "rtib": RTIB, "rtibo": {**RTIB, "ENOMOTO_LU_BTRAN_OUT": "1"}, "highs": None}
# Mittelmann LPopt のうち、以前の計測 (benchmarks/paper、16 スレッド) で当方・HiGHS・CLP (1 スレッドの単体法) のどれかが 600 秒以内に解いたもの
MITTEL = ["Linf_520c", "cont1", "datt256", "ex10", "fome13", "irish-electricity", "neos", "neos-5052403-cygnet", "neos-5251015",
          "ns1688926", "nug08-3rd", "pds-100", "physiciansched3-3", "qap15", "rail4284", "rmine15", "s250r10", "scpm1", "square41",
          "stormG2_1000", "supportcase10", "woodlands09"]


def emit(d):
    print("@@" + json.dumps(d), flush=True)


def worker(solver, mps, mode):
    if solver == "highs":
        import highspy
        h = highspy.Highs()
        h.setOptionValue("output_flag", False)
        h.readModel(mps)
        h.setOptionValue("time_limit", TL)
        if mode == "simplex":
            h.setOptionValue("solver", "simplex")
            h.setOptionValue("threads", 1)
        else:
            h.setOptionValue("threads", 4)
        t0 = time.perf_counter()
        h.run()
        t = time.perf_counter() - t0
        st = str(h.getModelStatus()).split(".")[-1]
        info = h.getInfo()
        emit({"time": t, "status": {"kOptimal": "optimal", "kInfeasible": "infeasible", "kUnbounded": "unbounded"}.get(st, st),
              "obj": h.getObjectiveValue() if st == "kOptimal" else None, "iters": int(info.simplex_iteration_count),
              "ipm_iters": int(info.ipm_iteration_count)})
        return
    from enomoto_solver import benchmark_highs as bh
    model = bh._read_our_model(Path(mps))[0]
    t0 = time.perf_counter()
    if mode == "simplex":
        out = model.solve(lp_solver="simplex", distinguish_infeasible_unbounded=True, threads=1, time_limit=TL, log=False)
    else:
        out = model.solve(lp_solver="auto", distinguish_infeasible_unbounded=True, threads=4, time_limit=TL, log=False)
    t = time.perf_counter() - t0
    emit({"time": t, "status": out["status"], "obj": out.get("objective")})


def parse_prof(err):
    """PROF_PHASES_EXT の表 (単体法の呼び出しごと) を足し合わせる。"""
    pr = {"calls": 0, "iters": 0, "wall_ms": 0.0}
    for l in err.splitlines():
        m = re.match(r"PROF_PHASES_EXT wall=([\d.]+)ms iters=(\d+)", l)
        if m:
            pr["calls"] += 1
            pr["wall_ms"] += float(m.group(1))
            pr["iters"] += int(m.group(2))
            continue
        m = re.match(r"\s+([a-z_0-9()]+)\s+([\d.]+)ms\s+[\d.]+% of wall", l)
        if m:
            k = m.group(1)
            pr[k] = pr.get(k, 0.0) + float(m.group(2))
    return pr


def run_once(solver, cfg, mps, mode):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    if solver == "ours":
        env.update(CFGS[cfg])
        env["ENOMOTO_PROF_PHASES_EXT"] = "1"
    if mode == "simplex":
        env.update({"RAYON_NUM_THREADS": "1", "OMP_NUM_THREADS": "1"})
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, __file__, "--worker", solver, str(mps), mode], env=env, capture_output=True, text=True,
                            timeout=TL + 300)
        out, err, rc = pr.stdout, pr.stderr, pr.returncode
    except subprocess.TimeoutExpired:
        return {"status": "killed", "time": TL, "wall": round(time.time() - t0, 1)}
    res = [json.loads(l[2:]) for l in out.splitlines() if l.startswith("@@")]
    r = res[-1] if res else {"status": "crash", "rc": rc, "err": err[-300:]}
    if solver == "ours":
        r["prof"] = parse_prof(err)
    r["wall"] = round(time.time() - t0, 1)
    return r


def problems(sel):
    if sel == "netlib":
        c = REPO / ".netlib_cache"
        names = [n for n in (c / "problems.txt").read_text().split() if n]
        return [(n, c / "mps" / f"{n}.mps") for n in names]
    if sel == "kennington":
        c = REPO / ".kennington_cache" / "mps"
        return [(p.stem, p) for p in sorted(c.glob("*.mps"))]
    if sel == "mittelmann":
        c = REPO / ".mittelmann_cache"
        out = []
        for n in MITTEL:
            ps = sorted(c.rglob(f"{n}.mps")) + sorted(c.rglob(f"{n}.mps.gz")) + sorted(c.rglob(f"{n}"))
            ps = [p for p in ps if p.is_file()]
            if ps:
                out.append((n, ps[0]))
            else:
                print(f"mittelmann: {n} が無い", file=sys.stderr)
        return out
    allp = dict(problems("netlib") + problems("kennington"))
    return [(n, allp[n]) for n in sel.split(",")]


def main():
    if sys.argv[1] == "--worker":
        return worker(sys.argv[2], sys.argv[3], sys.argv[4])
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("set")
    ap.add_argument("--shard", default="0/1")
    ap.add_argument("--mode", default="simplex", choices=["simplex", "auto4"])
    ap.add_argument("--cfgs", default="def,rtib,rtibo,highs")
    ap.add_argument("--tag", default=None)
    a = ap.parse_args()
    si, sn = map(int, a.shard.split("/"))
    tag = a.tag or f"{a.set if a.set in ('netlib', 'kennington', 'mittelmann') else 'sel'}_{a.mode}" + (f"_s{si}" if sn > 1 else "")
    outp = D / f"lpbench_{tag}.jsonl"
    done = {}
    if outp.exists():
        for l in outp.read_text().splitlines():
            r = json.loads(l)
            done.setdefault((r["problem"], r["cfg"]), []).append(r)
    cfgs = a.cfgs.split(",")
    reps = 3 if a.mode == "simplex" else 1
    commit = subprocess.run(["git", "-C", str(REPO), "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    for i, (name, mps) in enumerate(problems(a.set)):
        if i % sn != si:
            continue
        for rep in range(reps):
            order = cfgs[rep % len(cfgs):] + cfgs[:rep % len(cfgs)]
            for cfg in order:
                prev = done.get((name, cfg), [])
                if len(prev) > rep:
                    continue
                if prev and (prev[0]["result"].get("time", TL) >= 300 or prev[0]["result"].get("status") not in ("optimal", "infeasible", "unbounded")):
                    continue
                solver = "highs" if cfg == "highs" else "ours"
                r = run_once(solver, cfg, mps, a.mode)
                rec = {"problem": name, "cfg": cfg, "mode": a.mode, "rep": rep, "result": r, "commit": commit,
                       "at": time.strftime("%m-%d %H:%M:%S")}
                with open(outp, "a") as f:
                    f.write(json.dumps(rec) + "\n")
                done.setdefault((name, cfg), []).append(rec)
                p = r.get("prof", {})
                print(name, cfg, rep, r.get("status"), round(r.get("time", 0), 3), p.get("iters", r.get("iters")),
                      round(p.get("ftran", 0), 1), round(p.get("btran(rho_p)", 0), 1), flush=True)


if __name__ == "__main__":
    main()
