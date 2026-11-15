"""組 C の 7 問を 4 スレッド・長い時間の上限で 1 問ずつ解き、最大 RSS・RSS の推移・ログの大きさを残す。

使い方: python analysis/mittelmann_ready_20261103/long_c/run_long.py --kind ours|highs --time-limit 600 [--only NAME ...]
       [--logdir DIR]  (ENOMOTO のログ (ENOMOTO_MIP_LOG=1 の標準エラー) は大きいので既定は /tmp 側に置く)
/usr/bin/time が無い環境なので、os.wait4 の ru_maxrss (KiB) で最大 RSS を取り、5 秒ごとに /proc の VmRSS を記録する。
結果は同じディレクトリの results_<kind>_<tl>.json に問題ごとに書き足す。
"""
import argparse, json, os, signal, subprocess, sys, time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
PROBS = ["dano3_5", "pg5_34", "radiationm18-12-05", "neos-4738912-atrato", "sct2", "neos-3627168-kasai", "gen-ip054"]


def rss_kib(pid):
    """pid とその子孫の VmRSS の合計 (KiB)。"""
    tot = 0
    todo = [pid]
    while todo:
        q = todo.pop()
        try:
            for l in open(f"/proc/{q}/status"):
                if l.startswith("VmRSS:"):
                    tot += int(l.split()[1])
            for t in os.listdir(f"/proc/{q}/task"):
                todo += [int(c) for c in open(f"/proc/{q}/task/{t}/children").read().split()]
        except (FileNotFoundError, ProcessLookupError):
            pass
    return tot


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kind", choices=["ours", "highs"], required=True)
    ap.add_argument("--time-limit", type=float, default=600.0)
    ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--rss-cap-mib", type=float, default=0, help="RSS がこれを超えたら止める (手元の機械を守る。0 で止めない)")
    ap.add_argument("--logdir", type=Path, default=Path("/tmp/long_c_logs"))
    a = ap.parse_args()
    a.logdir.mkdir(parents=True, exist_ok=True)
    out = HERE / f"results_{a.kind}_{int(a.time_limit)}.json"
    res = json.loads(out.read_text()) if out.exists() else {}
    for n in a.only or PROBS:
        mps = REPO / ".miplib_cache" / f"{n}.mps"
        env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads))
        if a.kind == "ours":
            env["ENOMOTO_MIP_LOG"] = "1"
        log = a.logdir / f"{n}_{a.kind}_{int(a.time_limit)}.log"
        cmd = [sys.executable, str(REPO / "scripts/run_miplib_benchmark.py"), f"--worker-{a.kind}", str(mps),
               "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
        t0 = time.perf_counter()
        with open(log, "w") as ferr:
            p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=ferr, text=True, env=env)
            series = []
            killed = False
            while True:
                pid, st, ru = os.wait4(p.pid, os.WNOHANG)
                if pid:
                    break
                el = time.perf_counter() - t0
                series.append([round(el, 1), rss_kib(p.pid), log.stat().st_size])
                if a.rss_cap_mib and series[-1][1] > a.rss_cap_mib * 1024 and not killed:
                    p.send_signal(signal.SIGKILL); killed = "rss_cap"
                if el > a.time_limit * 1.5 + 60 and not killed:
                    p.send_signal(signal.SIGKILL); killed = True
                time.sleep(5)
            wall = time.perf_counter() - t0
            p.returncode = os.waitstatus_to_exitcode(st)
            stdout = p.stdout.read()
        r = {}
        for l in reversed(stdout.splitlines()):
            if l.startswith("{"):
                r = json.loads(l); break
        if not r:
            r = {"status": (f"killed ({killed})" if isinstance(killed, str) else "killed") if killed else f"crash rc={p.returncode}"}
        r.update(wall=wall, maxrss_kib=ru.ru_maxrss, log_bytes=log.stat().st_size, rss_series=series,
                 mip_warnings=open(log, errors="replace").read().count("MIP: WARNING") if a.kind == "ours" else None)
        res[n] = r
        out.write_text(json.dumps(res, indent=1))
        print(f"{n:24s} {r.get('status')} t={r.get('time')} wall={wall:.1f} obj={r.get('objective')} bound={r.get('bound')} "
              f"nodes={r.get('nodes')} maxrss={ru.ru_maxrss/1024:.0f}MiB log={r['log_bytes']/1e6:.1f}MB", flush=True)


if __name__ == "__main__":
    main()
