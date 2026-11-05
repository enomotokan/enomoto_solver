== loss21
  base            14.02s (20/21)
  scale           14.71s (18/21)
== miplib40
  base            18.79s (26/40)
  scale           19.23s (26/40)

問題ごと (各回の時間、base と 20% 以上かつ 2 秒以上違うもの):
  mc11                   rep0 base=14.2  scale=31.3
  misc07                 rep0 base=9.1  scale=15.4
  neos-3083819-nubu      rep0 base=10.2  scale=6.7
  neos17                 rep0 base=10.6  scale=7.4
  noswot                 rep0 base=26.5  scale=53.2
  p200x1188c             rep0 base=3.8  scale=5.9

食い違い (どれかが optimal で目的値が違う) と異常終了:
