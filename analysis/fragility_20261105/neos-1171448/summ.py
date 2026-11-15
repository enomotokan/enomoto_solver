"""log_*.txt から 1 行ずつの要約 (時間・ノード・暫定解の時刻と出どころ・ALNS の呼び出し) を出す。
使い方: python summ.py [タグの接頭辞 ...]"""
import json, re, sys, glob, os, collections
d = os.path.dirname(os.path.abspath(__file__))
pre = sys.argv[1:] or [""]
for f in sorted(glob.glob(os.path.join(d, "log_*.txt")), key=lambda f: (re.sub(r"\d+", "", os.path.basename(f)), int((re.findall(r"\d+", os.path.basename(f)) or ["0"])[-1]))):
    tag = os.path.basename(f)[4:-4]
    if not any(tag.startswith(p) for p in pre):
        continue
    L = open(f).read().splitlines()
    try:
        r = json.loads(L[-1])
    except Exception:
        print(tag, "未完"); continue
    inc = [re.search(r"new incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*)\]", l) for l in L]
    inc = [m for m in inc if m]
    ts = [l for l in L if l.startswith("MIP: tree search with")]
    t_tree = re.search(r"\((\S+)s\)", ts[0]).group(1) if ts else "-"
    al = [l for l in L if l.startswith("FRAG: alns")]
    arms = collections.Counter()
    for l in al:
        m = re.search(r"tid (\d+) role (\w+) .* arm (\d)", l)
        arms[("R" if m.group(2) == "true" else "T") + m.group(3)] += 1
    roles = [l for l in L if l.startswith("HEURROLE:")]
    print(f"{tag:5s} {r['time']:6.1f}s nodes {r['nodes']:5d} tree@{t_tree}s alns {len(al):3d} {dict(sorted(arms.items()))} role-exits {len(roles)}")
    for m in inc:
        print(f"      {float(m.group(1)):9.3f} node {m.group(2):>5s} {m.group(3):>7s}s {m.group(4)}")
