"""RENS (more nodes) のやり直しを 1 本ずつ出す (ENOMOTO_MIP_SUBMIP_VERBOSE の SMT 行から)。
使い方: python retries.py ログのディレクトリ [ファイル名の glob]
各やり直しについて、同じ親の直前の RENS (並列でも主でも) のノード数・上限・状態・解、やり直しのノード数・上限・解・親を良くしたか (acc)。
full = 直前の RENS が本当にノード上限まで進んだか (nodes >= node_limit)。"""
import gzip, re, sys
from pathlib import Path

BEG = re.compile(r"SMT begin id=(\d+) parent=(\S+) depth=(\d+) kind=\[(.*?)\] caller_node=(\d+) free=\S+ node_limit=(\d+) time_limit=(\S+) parallel=(\d)")
END = re.compile(r"SMT end id=(\d+) status=(\w+) nodes=(\d+) objective=(\S+) secs=(\S+)")
ACC = re.compile(r"SMT acc id=(\d+) accepted=(\d)")


def opener(f):
    return gzip.open(f, "rt", errors="replace") if str(f).endswith(".gz") else open(f, errors="replace")


def parse(f):
    S, order = {}, []
    with opener(f) as fh:
        for line in fh:
            m = BEG.search(line)
            if m:
                i = int(m[1])
                S[i] = dict(id=i, parent=m[2], depth=int(m[3]), kind=m[4], limit=int(m[6]), par=int(m[8]))
                order.append(i)
                continue
            m = END.search(line)
            if m and int(m[1]) in S:
                S[int(m[1])].update(status=m[2], nodes=int(m[3]), obj=m[4], secs=float(m[5]))
                continue
            m = ACC.search(line)
            if m and int(m[1]) in S:
                S[int(m[1])]["acc"] = int(m[2])
    return S, order


def retries(f):
    S, order = parse(f)
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
        out.append((s, prev))
    return out


if __name__ == "__main__":
    d = Path(sys.argv[1])
    pat = sys.argv[2] if len(sys.argv) > 2 else "*.log*"
    tot = {}
    for f in sorted(d.glob(pat)):
        for s, p in retries(f):
            full = p is not None and p.get("nodes", -1) >= p["limit"]
            key = (s["depth"], full)
            a = tot.setdefault(key, [0, 0.0, 0])
            a[0] += 1; a[1] += s.get("secs", 0.0); a[2] += s.get("acc", 0)
            ps = f"{p.get('status')} {p.get('nodes')}/{p['limit']} obj={p.get('obj')}" if p else "-"
            print(f"{f.name[:40]:40s} d={s['depth']} full={int(full)} prev: {ps} | retry: {s.get('status')} {s.get('nodes')}/{s['limit']} obj={s.get('obj')} {s.get('secs', 0):.2f}s acc={s.get('acc')}")
    print("\n深さ full 本数 秒 acc")
    for k in sorted(tot):
        print(k[0], int(k[1]), tot[k][0], f"{tot[k][1]:.1f}", tot[k][2])
