"""HANG の設定 (analysis/hang_20261117/README.md)。名前が _t1 で終われば 1 スレッド (run.py の TH)。
- def / def_t1: このコミットの既定 (期限の直し 3 つ + 早めの探索の 1 スレッドで連鎖の手の手間を数える直しを含む)。
- sst: 対称性を orbital fixing の代わりに Schreier-Sims の表のカットで扱う試作 (ENOMOTO_MIP_SYM_SST=1)。
- nct_t1: 1 スレッドの早めの探索で連鎖の手の手間を数えない (直す前と同じ、ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS=1)。
  def_t1 / nct_t1 で連鎖の手を数える直し (既定の 1 スレッドの道筋が変わる唯一の直し) の効き目を見る。
- rfs30 / rfs30_t1: 実行可能性問題 (目的が全部 0) の根の後のやり直し FJ に 30 秒の上限 (ENOMOTO_T_MIP_ROBUST_FJ_FEAS_SECS=30、既定は上限なし = 残りの 0.5)。
  効くのは目的が全部 0 の 7 問だけ (Mittelmann 186 問 + MIPLIB 58 問で数えた、README 10 節) なので shards_F.json で見る。"""
CFGS = {
    "def": {},
    "def_t1": {},
    "sst": {"ENOMOTO_MIP_SYM_SST": "1"},
    "sst_t1": {"ENOMOTO_MIP_SYM_SST": "1"},
    "nct_t1": {"ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS": "1"},
    # rin = 前処理の LP による固定の最初の LP など、反復の上限つきの根の LP の競争で二段解法が上限に当たったら内点法側を待たない (README 16 節)
    "rin": {"ENOMOTO_MIP_RACE_ITERCAP_NOWAIT": "1"},
    "rfs30": {"ENOMOTO_T_MIP_ROBUST_FJ_FEAS_SECS": "30"},
    "rfs30_t1": {"ENOMOTO_T_MIP_ROBUST_FJ_FEAS_SECS": "30"},
}
