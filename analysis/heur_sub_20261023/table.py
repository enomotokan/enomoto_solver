"""logs/<問題>_<設定>_<n>.log から表を作る: python table.py <問題> [設定 ...]
列: 設定・回・最初の暫定解 (値・時刻・出どころ)・最終 (状態・値)・各試作ヒューリスティクスの 呼んだ回数/解を出した回数"""
import json, re, sys, glob

prob = sys.argv[1]
cfgs = sys.argv[2:]
NAMES = [("Farkas diving", "Fk"), ("zeroobj", "ZO"), ("ACS", "ACS"), ("LB2", "LB2"), ("PROX2", "PX2")]


def one(f):
    txt = open(f).read().splitlines()
    try:
        js = json.loads(txt[-1])
    except Exception:
        js = {}
    inc = [re.search(r'new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[(.*)\]', l) for l in txt if 'MIP: new incumbent' in l]
    inc = [m for m in inc if m]
    # 最初の外側の暫定解 (サブ MIP の行は出ない: サブ MIP は verbose でない)
    first = f"{float(inc[0].group(1)):.6g} @{float(inc[0].group(2)):.1f}s [{inc[0].group(3)}]" if inc else '-'
    cnt = []
    for key, short in NAMES:
        ls = [l for l in txt if l.startswith(f'MIP: hsub {key}:') and '-> found' in l]
        nf = sum('found true' in l for l in ls)
        cnt.append(f"{len(ls)}/{nf}" if ls else '-')
    # 試作のヒューリスティクスが出した暫定解 (外側のラベル)
    by = [m for m in inc if re.search(r'ACS|zero objective|proximity 2|local branching 2|Farkas', m.group(3))]
    obj = js.get('objective')
    fin = f"{js.get('status')} {'-' if obj is None else f'{obj:.6g}'}"
    return first, fin, cnt, len(by), js


for c in cfgs:
    for f in sorted(glob.glob(f'logs/{prob}_{c}_*.log'), key=lambda s: int(re.search(r'_(\d+)\.log$', s).group(1))):
        n = re.search(r'_(\d+)\.log$', f).group(1)
        first, fin, cnt, nby, js = one(f)
        print(f"| {c} | {n} | {first} | {fin} | " + " | ".join(cnt) + f" | {nby} |")
