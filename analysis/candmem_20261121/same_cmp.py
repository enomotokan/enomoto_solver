"""same.sh の書き出しを比べる: 問題ごとに、old と new の根のラウンドの候補のファイル (cands_r<k>.txt) が先頭から何ラウンド同じか。
根の LP の解 (4 スレッドの競走) やその後の道が分かれると違うのは当たり前なので、「最初に違うラウンドの前のラウンドまで同じか」と、
最初に違うラウンドの直前の LP の値 (ログの cut round 行) を並べる。使い方: python same_cmp.py <名札> [問題…]"""
import hashlib, os, re, sys
tag = sys.argv[1]
for p in sys.argv[2:] or sorted(os.listdir("same")):
    d = {b: f"same/{p}/{tag}_{b}" for b in ("old", "new")}
    if not all(os.path.isdir(v) for v in d.values()):
        continue
    def files(b):
        return {int(re.findall(r"\d+", f)[0]): f for f in os.listdir(d[b]) if f.startswith("cands_r")}
    fo, fn = files("old"), files("new")
    same = 0
    first_diff = None
    for k in sorted(set(fo) | set(fn)):
        if k not in fo or k not in fn:
            first_diff = (k, "片方だけ")
            break
        h = [hashlib.md5(open(f"{d[b]}/cands_r{k}.txt", "rb").read()).hexdigest() for b in ("old", "new")]
        if h[0] != h[1]:
            first_diff = (k, "違う")
            break
        same += 1
    n = [sum(1 for _ in open(f"{d['old']}/cands_r{k}.txt")) for k in sorted(fo)[:1]]
    def obj(b):
        return [l.split("obj ")[1].split()[0] for l in open(f"{d[b]}/run.log") if re.search(r"MIP: cut round \d+: \d+ cuts", l) and "background" not in l][:same + 2]
    print(f"{p}: ラウンド old {len(fo)} new {len(fn)}、先頭から同じ {same}、最初の違い {first_diff}、r0 の候補 {n}")
    print(f"   old obj {obj('old')}\n   new obj {obj('new')}")
