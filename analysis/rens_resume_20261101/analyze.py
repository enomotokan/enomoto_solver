"""RENS のやり直し (`RENS (more nodes)`) と継続 (ENOMOTO_MIP_RENS_RESUME) の集計 (2026-11-01)。`SMT ...` の行 (ENOMOTO_MIP_SUBMIP_VERBOSE) を読む。
使い方:
  python analyze.py retries ログ...   やり直し・継続を 1 本ずつ (直前の RENS、時間、根の割合、解、親を良くしたか)
  python analyze.py summary ログのディレクトリ [設定...]   設定 × 問題ごとに 3 回の結果 (状態・目的値・時間) とやり直しの時間の和
根の割合: やり直しのサブ MIP の始め (SMT begin の T) から、その中の木の始め (`SMT tree tag=.../id` の最初の T) までの秒 / やり直しの秒。
木に入らずに終わったものは全部を根に数える。「入れ子」はやり直しの根の間に始めた子のサブ MIP の秒の和 (根の秒の内訳)。
継続 (`SMT resume id=…`) は前処理・根をやり直さないので、木の始めの行はなく、根の割合は 0 とする。"""
import gzip, json, re, sys
from collections import defaultdict
from pathlib import Path

BEG = re.compile(r"SMT begin id=(\d+) parent=(\S+) depth=(\d+) kind=\[(.*?)\] caller_node=(\d+) free=\S+ node_limit=(\d+) time_limit=(\S+) parallel=(\d) T=(\S+)")
END = re.compile(r"SMT end id=(\d+) status=(\w+) nodes=(\d+) objective=(\S+) secs=(\S+) T=(\S+)")
ACC = re.compile(r"SMT acc id=(\d+) accepted=(\d)")
TREE = re.compile(r"SMT tree tag=(\S+) restarts=\d+ secs=\S+ T=(\S+)")
RES = re.compile(r"SMT resume id=(\d+) from_nodes=(\d+)")


def opener(f):
    return gzip.open(f, "rt", errors="replace") if str(f).endswith(".gz") else open(f, errors="replace")


def parse(f):
    S, order, tree, result = {}, [], {}, None
    with opener(f) as fh:
        for line in fh:
            if line.startswith("{") and '"status"' in line:
                try:
                    result = json.loads(line)
                except Exception:
                    pass
                continue
            if "SMT " not in line:
                continue
            m = BEG.search(line)
            if m:
                i = int(m[1])
                S[i] = dict(id=i, parent=m[2], depth=int(m[3]), kind=m[4], limit=int(m[6]), tl=float(m[7]), par=int(m[8]), T0=float(m[9]), resume=None)
                order.append(i)
                continue
            m = END.search(line)
            if m and int(m[1]) in S:
                S[int(m[1])].update(status=m[2], nodes=int(m[3]), obj=m[4], secs=float(m[5]), T1=float(m[6]), paused="paused=1" in line)
                continue
            m = ACC.search(line)
            if m and int(m[1]) in S:
                S[int(m[1])]["acc"] = int(m[2])
                continue
            m = RES.search(line)
            if m and int(m[1]) in S:
                S[int(m[1])]["resume"] = int(m[2])
                continue
            m = TREE.search(line)
            if m:
                last = m[1].rsplit("/", 1)[-1]
                if last.isdigit() and int(last) not in tree:
                    tree[int(last)] = float(m[2])
    return S, order, tree, result


def retries(f):
    S, order, tree, result = parse(f)
    out = []
    for k, i in enumerate(order):
        s = S[i]
        if s["kind"] != "RENS (more nodes)":
            continue
        prev = None
        for j in reversed(order[:k]):
            t = S[j]
            if t["parent"] == s["parent"] and t["kind"] == "RENS":
                prev = t
                break
        secs = s.get("secs", 0.0)
        if s["resume"] is not None:
            root, nest = 0.0, 0.0
        else:
            t1 = tree.get(i, s.get("T1", s["T0"] + secs))
            root = max(0.0, min(t1, s["T0"] + secs) - s["T0"])
            path = f"/{i}"
            nest = sum(c.get("secs", 0.0) for c in S.values() if c["parent"].endswith(path) and c["T0"] < t1)
        out.append(dict(s=s, prev=prev, secs=secs, root=root, nest=nest))
    return out, result


def fmt_obj(o):
    if o is None or o == "None":
        return "-"
    m = re.search(r"-?[0-9][-0-9.eE+]*", o)
    return f"{float(m[0]):.6g}" if m else o


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "retries":
        tot = defaultdict(lambda: [0, 0.0, 0.0, 0.0, 0])
        for f in sys.argv[2:]:
            rs, _ = retries(f)
            for r in rs:
                s, p = r["s"], r["prev"]
                full = p is not None and p.get("nodes", -1) >= p["limit"]
                kind = "resume" if s["resume"] is not None else "redo"
                ps = f"{p.get('status')} {p.get('nodes')}/{p['limit']} {fmt_obj(p.get('obj'))}" if p else "-"
                print(f"{Path(f).name[:44]:44s} d={s['depth']} {kind:6s} full={int(full)} prev: {ps:28s} | {s.get('status')} {s.get('nodes')}/{s['limit']} {fmt_obj(s.get('obj')):>10s} {r['secs']:6.2f}s root {r['root']:5.2f}s (nested {r['nest']:5.2f}s) acc={s.get('acc')}")
                a = tot[(s["depth"], kind, full)]
                a[0] += 1; a[1] += r["secs"]; a[2] += r["root"]; a[3] += r["nest"]; a[4] += s.get("acc", 0) or 0
        print("\n深さ 種類 full 本数 秒 根の秒 (入れ子) 根の割合 acc")
        for k in sorted(tot):
            a = tot[k]
            print(k[0], k[1], int(k[2]), a[0], f"{a[1]:.2f}", f"{a[2]:.2f}", f"({a[3]:.2f})", f"{a[2] / a[1]:.2f}" if a[1] > 0 else "-", a[4])
    elif mode == "summary":
        d = Path(sys.argv[2])
        confs = sys.argv[3:] or sorted(x.name for x in d.iterdir() if x.is_dir())
        probs = sorted({re.sub(r"_r\d+\.log(\.gz)?$", "", f.name) for c in confs for f in (d / c).glob("*.log*")})
        print("問題 | 設定 | 回ごとの (状態 目的値 時間) | やり直し・継続の本数 (外側の深さ 1) / 秒の和 | 全部の深さの本数 / 秒の和")
        for p in probs:
            for c in confs:
                cells, n1, s1, na, sa = [], 0, 0.0, 0, 0.0
                for f in sorted((d / c).glob(f"{p}_r*.log*")):
                    rs, res = retries(f)
                    if res:
                        st = {"optimal": "opt", "time_limit": "TL"}.get(res.get("status"), res.get("status"))
                        cells.append(f"{st} {res.get('objective', float('nan')):.6g} {res.get('time', 0):.1f}s")
                    else:
                        cells.append("NO RESULT")
                    for r in rs:
                        na += 1; sa += r["secs"]
                        if r["s"]["depth"] == 1:
                            n1 += 1; s1 += r["secs"]
                print(f"{p} | {c} | {' ; '.join(cells)} | {n1} / {s1:.2f}s | {na} / {sa:.2f}s")
