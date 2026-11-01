"""neos-960392 で設定を変えた試し (2026-11-01、既定 4 スレッド・60 秒、ソルバーは同時に 1 つ)。
使い方: python run.py <設定名,...> <問題,...> <回数> [時間上限 (秒、既定 60)]。
ログは <問題>_<設定>[_t<秒>]_<回>.log、1 行の結果は results.jsonl に足す。
設定名の後ろに @<スレッド数> を付けるとスレッド数を変える (例: def@8)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
C = WT / ".miplib_cache"
TRIM3 = {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_RESUME": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"}
CFGS = {
    "def": {},
    "trim3": TRIM3,
    # 旧既定 (analysis/cand_default_20261031/olddef.env)
    "olddef": {"ENOMOTO_MIP_SUBMIP_HIGHS": "0", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "0", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "-1", "ENOMOTO_MIP_BG_CUTS_WAIT": "0",
               "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "0", "ENOMOTO_T_MIP_RESTART_FIRST_FAC": "0", "ENOMOTO_MIP_BG_CUTS_PROOF_STOP": "0"},
    # 再スタートを止める (初回の閾値を 100%)
    "norst": {"ENOMOTO_T_MIP_RESTART_FIRST_FAC": "1"},
    # 裏のカットを待たない
    "bgw0": {"ENOMOTO_MIP_BG_CUTS_WAIT": "0"},
    # 根の LP を内点法 + クロスオーバーの頂点で決める
    "ipm": {"ENOMOTO_MIP_ROOT_LP_IPM": "1"},
    # サブ MIP を旧来の形に
    "sh0": {"ENOMOTO_MIP_SUBMIP_HIGHS": "0"},
    # 前処理の LP による固定の最初の LP を待たない (旧来の打ち切り)
    "fin0": {"ENOMOTO_MIP_LP_PROBE_FINISH": "0"},
    # 既定 + サブ MIP の打ち切りの理由をログに出す (診断のみ)
    "dbg": {"ENOMOTO_MIP_DEBUG_SUBMIP_STOP": "1"},
    # 根のサブ MIP の成果連動の打ち切り (解を見つけてから max(そこまでのノード, 50) ノード良くならなければ止める) を使わない
    "noout": {"ENOMOTO_MIP_NO_SUBMIP_OUTCOME_STOP": "1", "ENOMOTO_MIP_DEBUG_SUBMIP_STOP": "1"},
    # 成果連動の打ち切りの最小ノードを 50 -> 300
    "out300": {"ENOMOTO_T_MIP_SUBMIP_OUTCOME_MIN_NODES": "300", "ENOMOTO_MIP_DEBUG_SUBMIP_STOP": "1"},
    # 成果連動・停滞の打ち切りを両方使わない
    # 根のカットの時間の上限 (残り時間 x 0.1) を絞る・ラウンド数の上限 (25) を 3 に
    "cf06": {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.06"},
    "cf04": {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.04"},
    "cr3": {"ENOMOTO_T_MIP_CUT_ROUNDS": "3"},
    # 暫定解なし用の RENS を HiGHS の目標の固定率 (0.6) と lock の順に (1 節・2 節の別のセッションの確かめで 2/2)
    "lr06": {"ENOMOTO_T_MIP_NOINC_RENS_RATE": "0.6", "ENOMOTO_MIP_DIVE_LOCK_ORDER": "1"},
    # ここから 20ddf8f (trim3 が既定) + 試作 `ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS` のビルド。nd = 新しい既定
    "nd": {},
    "nd_cr3": {"ENOMOTO_T_MIP_CUT_ROUNDS": "3"},
    # 外側の根だけ 3 ラウンド (サブ MIP の中は 25 のまま)
    "nd_cr3o": {"ENOMOTO_T_MIP_CUT_ROUNDS": "3", "ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS": "25"},
    # サブ MIP の中だけ 3 ラウンド
    "nd_cr3s": {"ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS": "3"},
    "noout_st0": {"ENOMOTO_MIP_NO_SUBMIP_OUTCOME_STOP": "1", "ENOMOTO_T_MIP_SUBMIP_STALL": "0", "ENOMOTO_MIP_DEBUG_SUBMIP_STOP": "1"},
}
def run(cfg, prob, rep, tl):
    name, _, thr = cfg.partition("@")
    thr = thr or "4"
    tag = cfg.replace("@", "_th") + (f"_t{tl}" if tl != 60 else "")
    out = D / f"{prob}_{tag}_{rep}.log"
    if out.exists():
        return
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS=thr, **CFGS[name])
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{prob}.mps"),
                        "--time-limit", str(tl), "--threads", thr], cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 5 + 200)
    out.write_text(r.stderr + "\n" + r.stdout)
    last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else "{}"
    try:
        res = json.loads(last)
    except Exception:
        res = {"raw": last[:200]}
    rec = {"prob": prob, "cfg": cfg, "tl": tl, "rep": rep, "wall": round(time.time() - t0, 1), **{k: res.get(k) for k in ("status", "objective", "bound", "time", "nodes") if k in res}}
    with open(D / "results.jsonl", "a") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(json.dumps(rec, ensure_ascii=False), flush=True)
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    tl = int(sys.argv[4]) if len(sys.argv) > 4 else 60
    for rep in range(reps):
        for p in probs:
            for c in cfgs:
                run(c, p, rep, tl)
