"""parspeed (i) の集計: python summ_thr.py > summ_thr.txt (res_thr.jsonl)。
問題・種ごとに、木の 1 スレッドあたりのノード/秒 (木の秒 = 全体 - 根の秒、TRSTAT) を
- t1: 1 スレッド 1 本、c4: 同じ 1 スレッドを 4 本同時 (4 本の平均)、t4: 4 スレッド (全ノード / 木の秒 / 4) で出し、
  c4/t1 (同時に 4 本動くだけで落ちる分) と t4/t1 (並列の全体) を比べる。
- t4 の各スレッドの木の時間の内訳 (TREEPAR): 待ち (idle、うち最後の待ち tail)・木の中の LNS・同期 (sync)・役 (HEURROLE の in role) と、残り (ノードの処理)。
  残りの秒 / ノード (t4) と 木の秒 / ノード (t1) の比 = ノード 1 つの手間の比。
- ノードの最初の LP の反復/ノード (TREEPAR の dive・queue・pool の重み付き平均) を t1 と t4 で比べ、共有の待ち行列から取ったノードの反復も別に出す。
- CPU: t4 のプロセスの CPU 秒 / 壁の秒、スレッドの CPU の上位 4 本の和と残り (裏のスレッド) の和。"""
import json, math, re, collections
TREEPAR = re.compile(r"tid (\d+) ready ([\d.]+)s end ([\d.]+)s idle ([\d.]+)s \((\d+) waits, lock ([\d.]+)s sleep ([\d.]+)s, pool mean ([\d.]+)\) lns ([\d.]+)s tail ([\d.]+)s sync ([\d.]+)s nodes (\d+) \(lb>=opt (\d+)\) first LP \[dive (\d+) ([\d.]+)it ([\d.]+)ms \| queue (\d+) ([\d.]+)it ([\d.]+)ms \| pool (\d+) ([\d.]+)it ([\d.]+)ms\]")
ROLE = re.compile(r"^HEURROLE: tid (\d+) entry \d+ ([\d.]+)s in role")
GIVE = re.compile(r"nodes given away (\d+), taken (\d+)")
rows = [json.loads(l) for l in open("res_thr.jsonl") if l.strip()]
G = collections.defaultdict(list)
for r in rows: G[(r["problem"], r["seed"], r["mode"])].append(r)
def tree_secs(r):
    sp = r["split"]
    return (r["result"]["time"] - sp["root_secs"]) if sp["tree_started"] and r["result"].get("time") else None
def tp(r):
    out = []
    for l in r["log"]:
        m = TREEPAR.search(l)
        if m:
            v = m.groups()
            out.append({"tid": int(v[0]), "end": float(v[2]), "idle": float(v[3]), "waits": int(v[4]), "lns": float(v[8]), "tail": float(v[9]), "sync": float(v[10]), "nodes": int(v[11]),
                        "k": [(int(v[13]), float(v[14]), float(v[15])), (int(v[16]), float(v[17]), float(v[18])), (int(v[19]), float(v[20]), float(v[21]))]})
    return out
def role(r):
    d = collections.Counter()
    for l in r["log"]:
        m = ROLE.match(l)
        if m: d[int(m.group(1))] += float(m.group(2))
    return d
def lpit(ts, kinds=(0, 1, 2)):
    n = sum(t["k"][k][0] for t in ts for k in kinds); it = sum(t["k"][k][0] * t["k"][k][1] for t in ts for k in kinds)
    return it / n if n else None
def gm(xs):
    xs = [x for x in xs if x is not None and x > 0]
    return math.exp(sum(map(math.log, xs)) / len(xs)) if xs else None
keys = sorted({(p, s) for p, s, m in G if all((p, s, mm) in G for mm in ("t1", "t4", "c4"))})
print(f"行 {len(rows)}、3 形そろった (問題, 種) {len(keys)} 組")
hdr = "| 問題 | 種 | t1 ノード/秒 | c4/t1 | t4/t1 (1 本あたり) | t4 ノード比 | t4 の木の内訳 (待ち・LNS・役・同期の割合) | t4 ノード 1 つの手間 / t1 | LP 反復/ノード t1→t4 (共有から) | 渡す/取る | t4 CPU/壁 (裏のスレッドの CPU 秒) |"
print(hdr); print("|" + "---|" * hdr.count("|")[:-1] if False else "|" + "---|" * 11)
agg = collections.defaultdict(list)
for p, s in keys:
    a = G[(p, s, "t1")][0]; b = G[(p, s, "t4")][0]; cs = G[(p, s, "c4")]
    ta, tb = tree_secs(a), tree_secs(b)
    if not ta or not tb or ta < 0.5 or tb < 0.5 or a["result"]["nodes"] < 100: continue
    r1 = a["result"]["nodes"] / ta
    rc = [c["result"]["nodes"] / tree_secs(c) for c in cs if tree_secs(c) and tree_secs(c) > 0.5]
    c_ratio = (sum(rc) / len(rc)) / r1 if rc else None
    r4 = b["result"]["nodes"] / tb / 4
    T = tp(b); rl = role(b)
    tot = sum(t["end"] for t in T) or 4 * tb
    idle = sum(t["idle"] for t in T); lns = sum(t["lns"] for t in T); sync = sum(t["sync"] for t in T); rol = sum(rl.values())
    work = tot - idle - lns - sync - rol
    n4 = sum(t["nodes"] for t in T) or b["result"]["nodes"]
    per_node = (work / n4) / (ta / a["result"]["nodes"])
    T1 = tp(a)
    it1, it4, itp = lpit(T1), lpit(T), lpit(T, (2,))
    gv = next((GIVE.search(l).groups() for l in b["log"] if GIVE.search(l)), ("-", "-"))
    cpu = (b["cpu_user"] + b["cpu_sys"]) / b["wall"]; bg = sum(b["threads_cpu"][4:])
    f = lambda x: "-" if x is None else f"{x:.2f}"
    print(f"| {p} | {s} | {r1:.0f} | {f(c_ratio)} | {r4 / r1:.2f} | {b['result']['nodes'] / a['result']['nodes']:.2f} | {idle / tot:.2f}・{lns / tot:.2f}・{rol / tot:.2f}・{sync / tot:.3f} | {per_node:.2f} | "
          f"{f(it1)}→{f(it4)} ({f(itp)}) | {gv[0]}/{gv[1]} | {cpu:.2f} ({bg:.1f}) |")
    for k, v in (("c4/t1", c_ratio), ("t4/t1", r4 / r1), ("待ち", idle / tot), ("LNS", lns / tot), ("役", rol / tot), ("同期", sync / tot), ("手間", per_node),
                 ("反復比", (it4 / it1) if it1 and it4 else None), ("CPU/壁", cpu)):
        agg[k].append(v)
print()
for k, v in agg.items():
    vv = [x for x in v if x is not None]
    if not vv: continue
    if k in ("待ち", "LNS", "役", "同期"):
        print(f"{k}: 平均 {sum(vv) / len(vv):.3f} (最大 {max(vv):.3f}、{len(vv)} 組)")
    else:
        print(f"{k}: 幾何平均 {gm(vv):.3f} (最小 {min(vv):.2f}・最大 {max(vv):.2f}、{len(vv)} 組)")
