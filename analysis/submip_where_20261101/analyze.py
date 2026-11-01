"""サブ MIP がどこで解を見つけるかの集計 (2026-11-01)。logs/*.log(.gz) の `MIP: SMT ...` の行を読む。
使い方: python analyze.py [ログのディレクトリ] > summ.txt

`SMT` の行 (ENOMOTO_MIP_SUBMIP_VERBOSE=1 で出る、src/mip/heuristics.rs の submip_trace_begin など):
- begin id parent depth kind caller_node free node_limit time_limit parallel T: サブ MIP を解き始めた (parent は道筋、M は外側の求解)
- end id status nodes objective secs T: サブ MIP が終わった (secs はサブ MIP を解いた秒)
- acc id accepted T: 呼び出し側がサブ MIP の解を暫定解として受け入れたか (1 = 呼び出し側の暫定解が良くなった)
- inc tag z node src child t T: 道筋 tag の求解で暫定解が良くなった (node はその求解のノード数、src はヒューリスティクスの名、
  child はサブ MIP の解ならその番号)
"""
import gzip, json, re, sys
from collections import defaultdict
from pathlib import Path

D = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "logs"
KV = re.compile(r"(\w+)=(\[[^\]]*\]|\S+)")
BINS = [(0, 0, "0"), (1, 10, "1-10"), (11, 50, "11-50"), (51, 100, "51-100"), (101, 500, "101-500"), (501, 10**12, ">500")]


def nbin(n):
    for lo, hi, s in BINS:
        if lo <= n <= hi:
            return s


def parse(path):
    op = gzip.open if path.suffix == ".gz" else open
    subs, incs, result = {}, [], None
    with op(path, "rt", errors="replace") as f:
        for line in f:
            if line.startswith("{") and '"status"' in line:
                try:
                    result = json.loads(line)
                except Exception:
                    pass
                continue
            if not line.startswith("MIP: SMT "):
                continue
            w = line.split()
            ev = w[2]
            kv = {k: v for k, v in KV.findall(line)}
            if ev == "begin":
                i = int(kv["id"])
                subs[i] = dict(id=i, parent=kv["parent"], depth=int(kv["depth"]), kind=kv["kind"][1:-1], caller_node=int(kv["caller_node"]),
                               free=kv["free"], node_limit=int(kv["node_limit"]), par=kv["parallel"] == "1", T0=float(kv["T"]), end=None, acc=None)
            elif ev == "end":
                s = subs.get(int(kv["id"]))
                if s is not None:
                    s["end"] = dict(status=kv["status"], nodes=int(kv["nodes"]), secs=float(kv["secs"]), T=float(kv["T"]))
            elif ev == "acc":
                s = subs.get(int(kv["id"]))
                if s is not None:
                    s["acc"] = kv["accepted"] == "1"
            elif ev == "inc":
                src = kv["src"][1:-1]
                incs.append(dict(tag=kv["tag"], z=float(kv["z"]), node=int(kv["node"]), src=src, child=int(kv["child"]), t=float(kv["t"]), T=float(kv["T"])))
    # サブ MIP ごとの暫定解の列
    by = defaultdict(list)
    for e in incs:
        last = e["tag"].rsplit("/", 1)[-1]
        by[0 if last == "M" else int(last)].append(e)
    return subs, by, result


def kind_of(s):
    """サブ MIP の種類の名 (木の中のサブ MIP は呼び出し側のノード > 0 で区別する)。"""
    k = s["kind"] or "RINS (bg cuts, no label)"
    if s["caller_node"] > 0 and not k.startswith("tree"):
        k = "tree:" + k
    return k


def where(e, subs, by, chain=()):
    """暫定解の事象 e の出どころ: (分類, 説明, 最後の葉のノード)。入れ子のサブ MIP の解ならその中をたどる。"""
    if e["child"] > 0:
        c = subs.get(e["child"])
        ce = by.get(e["child"], [])
        if c is None or not ce:
            return ("submip?", f"sub-MIP #{e['child']} (no inc)", None)
        cls, desc, leaf = where(ce[-1], subs, by, chain + (e["child"],))
        return ("nested", f"{kind_of(c)}(d{c['depth']}) > {desc}", leaf)
    src = e["src"].replace("other thread: ", "")
    if e["node"] == 0:
        if src == "tree":
            return ("root LP integral", "node 0 LP integral", 0)
        return ("root heuristic", src, 0)
    if src == "tree":
        return ("tree LP integral", "tree LP integral", e["node"])
    return ("tree heuristic", src, e["node"])


def main():
    files = sorted(list(D.glob("*.log")) + list(D.glob("*.log.gz")))
    # 集計の器
    agg = defaultdict(lambda: dict(n=0, secs=0.0, acc=0, nosol=0, nosol_secs=0.0, nodes=0, toM=0))
    where_cnt = defaultdict(lambda: defaultdict(int))  # (kind,depth) -> 分類 -> 本数
    leaf_cnt = defaultdict(int)  # 受け入れられたサブ MIP の解の葉 (最後の出どころ): 分類/名
    nb = defaultdict(lambda: defaultdict(int))  # 分類 -> ノードの区分 -> 本数
    waste = defaultdict(lambda: [0, 0, 0.0, 0.0])  # (kind,depth) -> [本数, 最後の改善の後のノード, その後の秒, 全体の秒]
    waste_all = [0, 0, 0.0, 0.0]
    z0 = defaultdict(lambda: [0, 0.0])  # 最後の改善がノード 0 の深さ 1 のサブ MIP: 全体のノード数の区分 -> [本数, 最後の改善の後の秒]
    final_src = []
    per_prob = []
    for f in files:
        prob = f.name.split(".log")[0]
        subs, by, res = parse(f)
        # 外側 (M) に届いたサブ MIP: M の暫定解の事象の child から入れ子をたどる
        toM = set()
        def mark(i):
            if i in toM:
                return
            toM.add(i)
            ev = by.get(i, [])
            if ev and ev[-1]["child"] > 0:
                mark(ev[-1]["child"])
        for e in by.get(0, []):
            if e["child"] > 0:
                mark(e["child"])
        nacc1 = 0
        tsub = 0.0
        for i, s in subs.items():
            key = (kind_of(s), s["depth"])
            a = agg[key]
            a["n"] += 1
            secs = s["end"]["secs"] if s["end"] else 0.0
            a["secs"] += secs
            if s["depth"] == 1:
                tsub += secs
            a["nodes"] += s["end"]["nodes"] if s["end"] else 0
            ev = by.get(i, [])
            if not ev:
                a["nosol"] += 1
                a["nosol_secs"] += secs
            if s["acc"]:
                a["acc"] += 1
                if s["depth"] == 1:
                    nacc1 += 1
            if i in toM:
                a["toM"] += 1
            if s["acc"] and ev:
                last = ev[-1]
                cls, desc, leaf_node = where(last, subs, by)
                where_cnt[key][cls] += 1
                # 葉の分類 (入れ子をたどった最後)
                e = last
                while e["child"] > 0 and by.get(e["child"]):
                    e = by[e["child"]][-1]
                lcls, ldesc, _ = where(e, subs, by)
                if s["depth"] == 1:
                    leaf_cnt[(cls, lcls, ldesc if lcls in ("root heuristic", "tree heuristic") else "")] += 1
                    nb[cls if cls != "nested" else "nested(at node 0 of this sub-MIP)"][nbin(last["node"])] += 1
                if s["end"] and s["depth"] == 1 and last["node"] == 0:
                    z0[nbin(s["end"]["nodes"])][0] += 1
                    z0[nbin(s["end"]["nodes"])][1] += max(0.0, s["end"]["T"] - last["T"])
                if s["end"]:
                    w = waste[key]
                    w[0] += 1
                    w[1] += max(0, s["end"]["nodes"] - last["node"])
                    w[2] += max(0.0, s["end"]["T"] - last["T"])
                    w[3] += secs
                    if s["depth"] == 1:
                        waste_all[0] += 1
                        waste_all[1] += max(0, s["end"]["nodes"] - last["node"])
                        waste_all[2] += max(0.0, s["end"]["T"] - last["T"])
                        waste_all[3] += secs
        # 最終的な暫定解を最初に出したもの
        Mev = by.get(0, [])
        fs = None
        if Mev:
            zb = min(e["z"] for e in Mev)
            tol = 1e-6 * (1 + abs(zb))
            first = min((e for e in Mev if e["z"] <= zb + tol), key=lambda e: e["T"])
            cls, desc, leaf = where(first, subs, by)
            if first["child"] > 0:
                c = subs[first["child"]]
                who = f"sub-MIP {kind_of(c)}"
            elif first["node"] == 0:
                src = first["src"].replace("other thread: ", "")
                who = "main root: " + ("node 0 LP integral" if src == "tree" else src)
            else:
                src = first["src"].replace("other thread: ", "")
                who = "main tree: " + ("LP integral" if src == "tree" else src)
            e = first
            while e["child"] > 0 and by.get(e["child"]):
                e = by[e["child"]][-1]
            fs = dict(prob=prob, who=who, desc=desc, node=first["node"], t=first["T"], leaf_node=leaf, leaf_cls=where(e, subs, by)[0])
            final_src.append(fs)
        st = res["status"] if res else "?"
        per_prob.append((prob, st, res.get("time") if res else None, len(subs), sum(1 for s in subs.values() if s["depth"] == 1), nacc1, tsub, fs))

    print("# サブ MIP の種類と深さごと (本数・解いた秒の和・呼び出し側の暫定解を良くした本数 acc・外側 (主) の暫定解に届いた本数 toM・解なし)")
    print(f"| 種類 | 深さ | 本数 | 秒の和 | ノードの和 | acc | toM | 解なしの本数 | 解なしの秒 |")
    print("|---|---|---|---|---|---|---|---|---|")
    for key in sorted(agg, key=lambda k: (k[1], -agg[k]["secs"])):
        a = agg[key]
        print(f"| {key[0]} | {key[1]} | {a['n']} | {a['secs']:.1f} | {a['nodes']} | {a['acc']} | {a['toM']} | {a['nosol']} | {a['nosol_secs']:.1f} |")
    print()
    print("# 呼び出し側の暫定解を良くしたサブ MIP の、最後の改善の出どころ (種類・深さごと)")
    cls_names = ["root LP integral", "root heuristic", "nested", "tree heuristic", "tree LP integral", "submip?"]
    print("| 種類 | 深さ | " + " | ".join(cls_names) + " |")
    print("|---|---|" + "---|" * len(cls_names))
    for key in sorted(where_cnt, key=lambda k: (k[1], k[0])):
        print(f"| {key[0]} | {key[1]} | " + " | ".join(str(where_cnt[key].get(c, 0)) for c in cls_names) + " |")
    print()
    print("# 深さ 1 で外側の暫定解を良くしたサブ MIP: 出どころ (入れ子なら中の葉) の内訳")
    print("| 出どころ | 葉 | 葉の名 | 本数 |")
    print("|---|---|---|---|")
    for k, v in sorted(leaf_cnt.items(), key=lambda kv: -kv[1]):
        print(f"| {k[0]} | {k[1]} | {k[2]} | {v} |")
    print()
    print("# 深さ 1 で外側の暫定解を良くしたサブ MIP: 最後の改善のノード番号の分布 (出どころごと)")
    print("| 出どころ | " + " | ".join(b[2] for b in BINS) + " |")
    print("|---|" + "---|" * len(BINS))
    for c in sorted(nb):
        print(f"| {c} | " + " | ".join(str(nb[c].get(b[2], 0)) for b in BINS) + " |")
    print()
    print("# 呼び出し側の暫定解を良くしたサブ MIP の、最後の改善の後に回したノードと秒 (無駄の量)")
    print("| 種類 | 深さ | 本数 | 最後の改善の後のノード | その後の秒 | 全体の秒 |")
    print("|---|---|---|---|---|---|")
    for key in sorted(waste, key=lambda k: (k[1], -waste[k][2])):
        w = waste[key]
        print(f"| {key[0]} | {key[1]} | {w[0]} | {w[1]} | {w[2]:.1f} | {w[3]:.1f} |")
    print(f"| (深さ 1 の計) | 1 | {waste_all[0]} | {waste_all[1]} | {waste_all[2]:.1f} | {waste_all[3]:.1f} |")
    print()
    print("# 深さ 1 で外側の暫定解を良くし、最後の改善がノード 0 だったサブ MIP: 回した全ノード数の分布と、最後の改善の後の秒")
    print("| 全ノード数 | 本数 | 最後の改善の後の秒 |")
    print("|---|---|---|")
    for b in BINS:
        if b[2] in z0:
            print(f"| {b[2]} | {z0[b[2]][0]} | {z0[b[2]][1]:.1f} |")
    print()
    print("# 最終的な暫定解をサブ MIP が最初に出した問題: その解の葉 (入れ子をたどった最後) の出どころ")
    lc = defaultdict(list)
    for fs in final_src:
        if fs["who"].startswith("sub-MIP"):
            lc[fs["leaf_cls"]].append(f"{fs['prob']}({fs['leaf_node']})")
    print("| 葉の出どころ | 問題数 | 問題 (葉のノード) |")
    print("|---|---|---|")
    for k, v in sorted(lc.items(), key=lambda kv: -len(kv[1])):
        print(f"| {k} | {len(v)} | {', '.join(v)} |")
    print()
    print("# 最終的な暫定解 (最適解または最後の暫定解) を最初に出したもの")
    cnt = defaultdict(int)
    for fs in final_src:
        cnt[fs["who"]] += 1
    print("| 出したもの | 問題数 |")
    print("|---|---|")
    for k, v in sorted(cnt.items(), key=lambda kv: -kv[1]):
        print(f"| {k} | {v} |")
    print()
    print("# 問題ごと (状態・秒・サブ MIP の本数 (全部 / 深さ 1)・深さ 1 で受け入れた本数・深さ 1 の秒の和・最終的な暫定解の出どころ)")
    print("| 問題 | 状態 | 秒 | サブ MIP | 深さ 1 | acc | 深さ 1 の秒 | 最終解を最初に出したもの | その中の出どころ | ノード | 時刻 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for prob, st, t, n, n1, a1, ts, fs in per_prob:
        tt = f"{t:.1f}" if isinstance(t, (int, float)) else "?"
        if fs:
            print(f"| {prob} | {st} | {tt} | {n} | {n1} | {a1} | {ts:.1f} | {fs['who']} | {fs['desc']} | {fs['leaf_node'] if fs['leaf_node'] is not None else '-'} | {fs['t']:.1f} |")
        else:
            print(f"| {prob} | {st} | {tt} | {n} | {n1} | {a1} | {ts:.1f} | (解なし) | | | |")


if __name__ == "__main__":
    main()
