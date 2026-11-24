"""条件付き候補の試作で、効いた問題と害の出た問題を分けられそうな問題の特徴を、既定の求解の診断ログから測る (README 9 節)。

    python analysis/protoscreen_20261124/feat.py [--problems a,b,…] [--threads 1] [--seed 1] [--tl 60] [--tag def] [--env K=V ...] [--out feat.jsonl]

- 既定: P の 58 問 (solu.json の問題)、1 スレッド・種 1・60 秒、試作なし。ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_FEAT_DIAG=1000000000 (終わりの FEAT の行だけ) を足し、
  stderr の終わりの診断の行から特徴を拾って 1 問 1 行で書く。止めても同じコマンドで続きから回る (同じ (問題, tag) は飛ばす)。
- `--env K=V` で試作をつけて測れる (例: `--tag enum --env ENOMOTO_MIP_ENUM_PRESOLVE=all` で列挙の得、`--tag sbpb --env ENOMOTO_MIP_SB_PROBEBD=1` で
  強分枝の両方の子から置いた境界の数)。tag は行の名前。
- 診断の出力は道筋を変えないはずだが、念のため res_s*.jsonl の def_t1 (種 1) とノード数・状態を比べて `same_path` に書く (--threads 1・試作なしのとき)。
- 拾う特徴 (どれも求解中に数えられるもの。行が出なければ None):
  zero_obj (目的が全部 0)、prune_bound・prune_infeas・phi (閉じたノードのうち実行不能で閉じた割合 = prune_infeas / (prune_infeas + prune_bound))、
  degen (FEAT の双対退化の平均)、sb_sel・sb_degen_skip・sb_degen_share (分枝の選びのうち双対退化で強分枝を省いた割合)、sb_lps・sb_iters、
  node_lps・mean_rows・iters_per_lp・final_rows、restart_newly_fixed・restart_threshold・root_lp_secs (最初の restart check の行)、
  enum_fixed・enum_tightened・enum_pairs (列挙の前処理の行の合計、試作をつけたときだけ出る)、sbpb_bounds (SB_PROBEBD のときだけ意味がある)、
  objprop_pruned、root_gap (|根の下界 − 既知の最適値| / max(1, |最適値|))、first_solution_time・first_solution_rel (最初の解の相対ギャップ)、nodes・time・status。
"""
import argparse, glob, json, math, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("--problems"); ap.add_argument("--threads", default="1"); ap.add_argument("--seed", type=int, default=1)
ap.add_argument("--tl", type=float, default=60); ap.add_argument("--tag", default="def"); ap.add_argument("--env", nargs="*", default=[])
ap.add_argument("--out", default=str(D / "feat.jsonl"))
a = ap.parse_args()
SOLU = json.loads((D / "solu.json").read_text(encoding="utf-8"))
probs = a.problems.split(",") if a.problems else sorted(SOLU)
extra = dict(kv.split("=", 1) for kv in a.env)
OUT = Path(a.out)
done = {(r["problem"], r["tag"]) for r in (json.loads(l) for l in (OUT.read_text().splitlines() if OUT.exists() else []) if l.strip())}
# 既定の道筋との比べ (res_s*.jsonl の def_t1・種 1)
REF = {}
for f in glob.glob(str(D / "res_s*.jsonl")):
    for l in open(f, encoding="utf-8"):
        if l.strip():
            r = json.loads(l)
            if r["cfg"] == "def_t1" and r["seed"] == a.seed:
                REF[r["problem"]] = r["result"]

NUM = r"([-+0-9.eEinfNa]+)"


def num(s):
    try:
        return float(s)
    except ValueError:
        return None


def parse(err):
    f = {}
    for l in err.splitlines():
        if l.startswith("FEAT end") and "tid=0" in l:
            kv = dict(x.split("=", 1) for x in l.split()[2:] if "=" in x)
            f["zero_obj"] = kv.get("zero_obj") == "true"
            pb, pi = int(kv.get("prune_bound", 0)), int(kv.get("prune_infeas", 0))
            f.update(prune_bound=pb, prune_infeas=pi, phi=pi / (pi + pb) if pi + pb else None, degen=num(kv.get("degen", "nan")))
            # 1 ノードあたりの衝突の数 (SB_ADAPT の cpn の近い量。FEAT の conflicts は HiGHS 式の衝突解析で作った数)
            f["cpn"] = int(kv.get("conflicts", 0)) / (int(kv.get("nodes", 0)) + 1)
            # NODEROWS_ADAPT の φ (nra_share、(実行不能で閉じた葉 + 1) / (閉じた葉 + 2))。試作をつけなくても数えている
            if "nra_inf" in kv:
                ni, nb = int(kv["nra_inf"]), int(kv["nra_bound"])
                f.update(nra_inf=ni, nra_bound=nb, nra_phi=(ni + 1) / (ni + nb + 2))
        elif l.startswith("MIP: branching:") and "sb_sel" not in f:
            m = re.search(r"(\d+) selections, (\d+) without strong branching for dual degeneracy; sb LPs (\d+) iters (\d+)", l)
            if m:
                s, k, lp, it = map(int, m.groups())
                f.update(sb_sel=s, sb_degen_skip=k, sb_degen_share=k / s if s else None, sb_lps=lp, sb_iters=it)
        elif l.startswith("MIP: node diag:") and "node_lps" not in f:
            m = re.search(r"node LPs (\d+) mean rows " + NUM + r" iters (\d+) \(" + NUM + r"/LP\).*final rows (\d+)", l)
            if m:
                f.update(node_lps=int(m[1]), mean_rows=num(m[2]), iters_per_lp=num(m[4]), final_rows=int(m[5]))
        elif l.startswith("MIP: restart check:") and "restart_newly_fixed" not in f:
            m = re.search(r"newly fixed (\d+), integer columns (\d+), free at the root (\d+), threshold " + NUM + r", root LP " + NUM + "s", l)
            if m:
                f.update(restart_newly_fixed=int(m[1]), restart_threshold=num(m[4]), root_lp_secs=num(m[5]))
        elif l.startswith("MIP: enumeration presolve"):
            m = re.search(r"(\d+) fixed, (\d+) bounds tightened, (\d+) equivalent pairs, (\d+) new conflict pairs", l)
            if m:
                for k, v in zip(("enum_fixed", "enum_tightened", "enum_eqpairs", "enum_pairs"), map(int, m.groups())):
                    f[k] = f.get(k, 0) + v
        elif l.startswith("MIP: ENUM_ADAPT:"):
            # ENUM_ADAPT をつけたとき: 最初の周の s と、続けた・止めた回数
            m = re.search(r" s " + NUM + r" f " + NUM + r" -> (\w+)", l)
            if m:
                f.setdefault("enum_adapt_s1", num(m[1]))
                k = "enum_adapt_" + m[3]
                f[k] = f.get(k, 0) + 1
        elif l.startswith("MIP: node rows adapt"):
            # NODEROWS_ADAPT をつけたとき: 終わりの φ (nra_share) と f の平均
            m = re.search(r"share " + NUM + r", f now " + NUM + r", mean f " + NUM + r"\)", l)
            if m:
                f.update(nra_share=num(m[1]), nra_mean_f=num(m[3]))
        elif l.startswith("MIP: objective propagation:") and "objprop_pruned" not in f:
            m = re.search(r"pruned (\d+) nodes.*node bounds from both children (\d+)", l)
            if m:
                f.update(objprop_pruned=int(m[1]), sbpb_bounds=int(m[2]))
    return f


for p in probs:
    if (p, a.tag) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(extra)
    env.update(ENOMOTO_MIP_THREADS=a.threads, ENOMOTO_T_MIP_SEED=str(a.seed), ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_FEAT_DIAG="1000000000")
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(a.tl), "--threads", a.threads],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=a.tl + 300)
    try:
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    f = parse(pr.stderr)
    sol = SOLU.get(p, {})
    opt = sol.get("value") if sol.get("kind") in ("opt", "best") else None

    def rel(v):
        return abs(v - opt) / max(1.0, abs(opt)) if isinstance(v, (int, float)) and opt is not None and math.isfinite(v) and abs(v) < 1e19 else None
    f.update(root_gap=rel(res.get("root_bound")), first_solution_time=res.get("first_solution_time"), first_solution_rel=rel(res.get("first_solution_objective")),
             nodes=res.get("nodes"), time=res.get("time"), status=res.get("status"))
    ref = REF.get(p)
    same = None
    if ref is not None and a.threads == "1" and not extra:
        same = ref.get("status") == res.get("status") and ref.get("nodes") == res.get("nodes")
    row = {"problem": p, "tag": a.tag, "threads": a.threads, "seed": a.seed, "tl": a.tl, "env": extra, "feat": f, "same_path": same, "wall": round(time.time() - t0, 1)}
    with open(OUT, "a") as fo:
        fo.write(json.dumps(row) + "\n")
    print(p, a.tag, res.get("status"), same, {k: (round(v, 3) if isinstance(v, float) else v) for k, v in f.items() if k in ("phi", "sb_degen_share", "zero_obj", "root_gap", "iters_per_lp")}, flush=True)
