"""衝突グラフの辺の内訳 (CUTMIX clique edges、前処理後の最初の呼び出し) とナップサック行と GUB の重なり (CUTMIX types の最初の行) の表。
使い方: python count.py 接頭辞 設定 種  (例: python count.py t47 def 1)"""
import glob, sys
pref, cfg, seed = sys.argv[1], sys.argv[2], sys.argv[3]
print("| 問題 | 0-1 列 | 辺 (全部) | 行から | 含意だけ | knap | knapgub | knapgubcol |")
print("|---|---|---|---|---|---|---|---|")
for f in sorted(glob.glob(f"logs/{pref}_*_{cfg}_s{seed}.txt")):
    p = f.split(f"{pref}_", 1)[1].rsplit(f"_{cfg}_s{seed}", 1)[0]
    ty = ce = None
    for l in open(f):
        w = l.split()
        if ty is None and l.startswith("CUTMIX types"):
            ty = dict(zip(w[2::2], w[3::2]))
        if ce is None and l.startswith("CUTMIX clique"):
            ce = dict(zip(w[3::2], w[4::2]))
    ty, ce = ty or {}, ce or {}
    print(f"| {p} | {ce.get('bin', '-')} | {ce.get('total', '-')} | {ce.get('rows', '-')} | {ce.get('implonly', '-')} | {ty.get('knap', '-')} | {ty.get('knapgub', '-')} | {ty.get('knapgubcol', '-')} |")
