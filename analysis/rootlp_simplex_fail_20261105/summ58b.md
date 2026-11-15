== loss21
  base            15.05s (20/21)
  comb            14.43s (19/21)
== miplib40
  base            19.03s (27/40)
  comb            18.49s (26/40)

問題ごと (各回の時間、base と 20% 以上かつ 2 秒以上違うもの):
  mik-250-20-75-4        rep0 base=5.5  comb=7.6
  neos-860300            rep0 base=29.8  comb=23.3
  neos-911970            rep0 base=13.9  comb=9.0
  noswot                 rep0 base=50.1  comb=31.2
  qiu                    rep0 base=32.0  comb=24.5

食い違い (どれかが optimal で目的値が違う) と異常終了:
