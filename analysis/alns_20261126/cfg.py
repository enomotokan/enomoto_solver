"""alns の設定 (analysis/alns_20261126)。def: 今の既定。alns: 試作 ENOMOTO_MIP_ALNS=1 (木の大近傍探索を CP-SAT の 3 本の近傍に替える)。
名前が _t1 で終わるものは 1 スレッド (run.py の TH)、ほかは 4 スレッド。"""
ALNS = {"ENOMOTO_MIP_ALNS": "1"}
# alnss: 1 回を手間 LNS_CALL_MIN ずつに分けて近傍を選び直す版 (今の ENOMOTO_MIP_ALNS=1。pint の alns は分ける前の版 = 今の "w")
# alnsb: それに加えて 1 スレッドの予算を効き目のシグモイドで決める (b)
CFGS = {"def": {}, "alns": ALNS, "def_t1": {}, "alns_t1": ALNS, "alnss": ALNS, "alnsb": {"ENOMOTO_MIP_ALNS": "1b"},
        "alnsb_t1": {"ENOMOTO_MIP_ALNS": "1b"},
        # alnsc: alnsb と同じ環境変数で、ビルドが違う (サブ MIP では使わない・b の効き目を Φ = 主ギャップ × 未処理の重みと窓で測る、README 8 節)
        "alnsc": {"ENOMOTO_MIP_ALNS": "1b"}, "alnsc_t1": {"ENOMOTO_MIP_ALNS": "1b"},
        # def2: 今の既定 (pint の def は前のビルドの既定なので、名前を分ける)
        "def2": {}}
