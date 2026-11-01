"""深さ 3 の RENS・深さ 2 の RENS (LP before cuts) を止める試作の集計 (2026-11-01)。logs/<問題>_<形>_<回>.log.gz を読む。
使い方: python analyze.py > summ.txt

読む行 (ENOMOTO_MIP_SUBMIP_VERBOSE=1):
- `MIP: SMT begin/end/acc/inc ...` (analysis/submip_where_20261101/analyze.py と同じ)
- `MIP: SMT rooth tag=… restarts=… secs=…`: 道筋 tag の求解の根のヒューリスティクスの時間 (このとき足した行)
- 最後の JSON の行 (run_miplib_benchmark.py の結果)
"""
import gzip, json, re, statistics, sys
from collections import defaultdict
from pathlib import Path

D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from run import PROBS, CFGS, EXTRA

KV = re.compile(r"(\w+)=(\[[^\]]*\]|\S+)")


def parse(path):
    subs, incs, rooth, res = {}, [], [], None
    with gzip.open(path, "rt", errors="replace") as f:
        for line in f:
            if line.startswith("{") and '"status"' in line:
                try:
                    res = json.loads(line)
                except Exception:
                    pass
                continue
            if not line.startswith("MIP: SMT "):
                continue
            ev = line.split()[2]
            kv = dict(KV.findall(line))
            if ev == "begin":
                i = int(kv["id"])
                subs[i] = dict(depth=int(kv["depth"]), kind=kv["kind"][1:-1] or "RINS (bg cuts, no label)", node=int(kv["caller_node"]),
                               par=kv["parallel"] == "1", secs=0.0, nodes=0, acc=False, sol=False)
            elif ev == "end" and int(kv["id"]) in subs:
                s = subs[int(kv["id"])]
                s["secs"] = float(kv["secs"])
                s["nodes"] = int(kv["nodes"])
            elif ev == "acc" and int(kv["id"]) in subs:
                subs[int(kv["id"])]["acc"] = kv["accepted"] == "1"
            elif ev == "inc":
                incs.append(dict(tag=kv["tag"], z=float(kv["z"]), node=int(kv["node"]), src=kv["src"][1:-1], child=int(kv["child"]), t=float(kv["t"])))
            elif ev == "rooth":
                rooth.append(dict(tag=kv["tag"], restarts=int(kv["restarts"]), secs=float(kv["secs"])))
    by = defaultdict(list)
    for e in incs:
        last = e["tag"].rsplit("/", 1)[-1]
        by[0 if last == "M" else int(last)].append(e)
        if last != "M" and int(last) in subs:
            subs[int(last)]["sol"] = True
    return subs, by, rooth, res


def chain(e, subs, by):
    """外側の暫定解の事象 e の出どころを入れ子でたどった列 [(種類, 深さ), ...] と葉の名。"""
    out = []
    while e["child"] > 0 and e["child"] in subs:
        c = subs[e["child"]]
        out.append((c["kind"], c["depth"]))
        ev = by.get(e["child"], [])
        if not ev:
            break
        e = ev[-1]
    return out, e["src"], e["node"]


def run_stats(path):
    subs, by, rooth, res = parse(path)
    r = dict(status=res["status"] if res else "?", time=res["time"] if res else None, obj=res.get("objective") if res else None)
    r["sub_secs"] = sum(s["secs"] for s in subs.values() if s["depth"] == 1)  # 深さ 1 の和 (中の入れ子を含む)
    r["sub_secs_all"] = sum(s["secs"] for s in subs.values())  # 全部の深さの和 (入れ子は二重に数える)
    r["n1"] = sum(1 for s in subs.values() if s["depth"] == 1)
    for d in (2, 3):
        ss = [s for s in subs.values() if s["depth"] == d]
        r[f"n{d}"] = len(ss)
        r[f"s{d}"] = sum(s["secs"] for s in ss)
    d3 = [s for s in subs.values() if s["depth"] >= 3 and s["kind"].startswith("RENS")]
    r["d3rens_n"], r["d3rens_s"], r["d3rens_sol"] = len(d3), sum(s["secs"] for s in d3), sum(s["sol"] for s in d3)
    lb = [s for s in subs.values() if s["depth"] == 2 and s["kind"] == "RENS (LP before cuts)"]
    r["d2lpbc_n"], r["d2lpbc_s"], r["d2lpbc_acc"] = len(lb), sum(s["secs"] for s in lb), sum(s["acc"] for s in lb)
    d2r = [s for s in subs.values() if s["depth"] == 2 and s["kind"].startswith("RENS")]
    r["d2rens_n"], r["d2rens_s"] = len(d2r), sum(s["secs"] for s in d2r)
    mr = [x for x in rooth if x["tag"] == "M"]
    r["rooth0"] = sum(x["secs"] for x in mr if x["restarts"] == 0)
    r["rooth"] = sum(x["secs"] for x in mr)
    # 最終解を最初に出したもの
    Mev = by.get(0, [])
    r["final"] = "-"
    r["t_final"] = None
    if Mev:
        zb = min(e["z"] for e in Mev)
        first = min((e for e in Mev if e["z"] <= zb + 1e-6 * (1 + abs(zb))), key=lambda e: e["t"])
        ch, leaf, node = chain(first, subs, by)
        r["t_final"] = first["t"]
        if ch:
            r["final"] = " > ".join(f"{k}(d{d})" for k, d in ch) + f" > {leaf}@{node}"
        else:
            r["final"] = ("main root: " if first["node"] == 0 else "main tree: ") + first["src"].replace("other thread: ", "")
    return r


def fmt(v, nd=1):
    return "-" if v is None else f"{v:.{nd}f}"


def main():
    files = sorted((D / "logs").glob("*.log.gz"))
    runs = defaultdict(list)
    for f in files:
        p, c, k = f.name[: -len(".log.gz")].rsplit("_", 2)
        runs[(p, c)].append((int(k), run_stats(f)))
    cfgs = [c for c in CFGS if any(k[1] == c for k in runs)]
    print("# 問題・形ごとの 3 回の平均 (秒は run_miplib_benchmark.py の time。T は時間切れの回数。sub は深さ 1 のサブ MIP の秒の和 (入れ子を含む)、")
    print("# rh は外側の最初の根のヒューリスティクスの秒 (rh+ は再スタート後の根も含む)、d3 は深さ 3 の RENS の本数/秒、lpbc は深さ 2 の RENS (LP before cuts) の本数/秒)")
    print()
    print("| 問題 | 形 | 秒 | T | 目的値 (3 回) | 最終解の時刻 | sub | rh | rh+ | d2 RENS 本/秒 | d3 本/秒 | lpbc 本/秒 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    tot = defaultdict(lambda: defaultdict(float))
    for p in PROBS + EXTRA:
        if p in EXTRA and p == EXTRA[0]:
            print('| (追加の 2 問。下の 11 問の和には入れない) |||||||||||')
        for c in cfgs:
            rs = [r for _, r in sorted(runs.get((p, c), []))]
            if not rs:
                continue
            m = lambda k: statistics.mean(r[k] for r in rs if r[k] is not None) if any(r[k] is not None for r in rs) else None
            nT = sum(r["status"] != "optimal" for r in rs)
            objs = "/".join("-" if r["obj"] is None else f"{r['obj']:.6g}" for r in rs)
            print(f"| {p} | {c} | {fmt(m('time'))} | {nT} | {objs} | {fmt(m('t_final'))} | {fmt(m('sub_secs'))} | {fmt(m('rooth0'), 2)} | {fmt(m('rooth'), 2)} | "
                  f"{m('d2rens_n'):.1f}/{m('d2rens_s'):.1f} | {m('d3rens_n'):.1f}/{m('d3rens_s'):.2f} | {m('d2lpbc_n'):.1f}/{m('d2lpbc_s'):.2f} |")
            if p in EXTRA:
                continue
            for k in ("time", "sub_secs", "rooth0", "rooth", "d3rens_n", "d3rens_s", "d3rens_sol", "d2lpbc_n", "d2lpbc_s", "d2lpbc_acc", "d2rens_n", "d2rens_s"):
                tot[c][k] += m(k) or 0.0
    print()
    print("# 11 問の和 (3 回の平均の和)")
    print("| 形 | 秒 | sub | rh | rh+ | d2 RENS 本/秒 | d3 RENS 本/秒/解あり | lpbc 本/秒/acc |")
    print("|---|---|---|---|---|---|---|---|")
    for c in cfgs:
        t = tot[c]
        print(f"| {c} | {t['time']:.1f} | {t['sub_secs']:.1f} | {t['rooth0']:.2f} | {t['rooth']:.2f} | {t['d2rens_n']:.1f}/{t['d2rens_s']:.1f} | "
              f"{t['d3rens_n']:.1f}/{t['d3rens_s']:.2f}/{t['d3rens_sol']:.1f} | {t['d2lpbc_n']:.1f}/{t['d2lpbc_s']:.2f}/{t['d2lpbc_acc']:.1f} |")
    print()
    print("# 回ごとの最終解の出どころ (最終の目的値を外側が最初に持った事象。入れ子は > でたどる、@ は葉のノード)")
    for p in PROBS + EXTRA:
        for c in cfgs:
            for k, r in sorted(runs.get((p, c), [])):
                print(f"{p} {c} {k}: {r['status']} time={fmt(r['time'])} obj={r['obj']} t_final={fmt(r['t_final'])} {r['final']}")


if __name__ == "__main__":
    main()
