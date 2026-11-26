# 診断 3: 直し c (子の上限を効き目のシグモイドで縮める) の確かめ。d と dc を 1 スレッド・種 1 で、木に入ったところで止める
for p in neos-3083819-nubu neos-860300 flugpl dcmulti blend2 cod105 cvs16r128-89 physiciansched6-2 graph20-20-1rand qiu 10teams 30n20b8; do
  DIAG_STOP_TREE=1 python diag.py res_diag3.jsonl $p 1 1 60 ENOMOTO_MIP_LPPROBE_FIX=d
  DIAG_STOP_TREE=1 python diag.py res_diag3.jsonl $p 1 1 60 ENOMOTO_MIP_LPPROBE_FIX=dc
done
