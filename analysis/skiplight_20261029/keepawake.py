"""計測中に画面消灯からの Modern Standby に入らないよう、ES_DISPLAY_REQUIRED を保ち続ける (計測が終わったら止める)。"""
import ctypes, subprocess, time
from pathlib import Path
D = Path(__file__).resolve().parent
ctypes.windll.kernel32.SetThreadExecutionState(0x80000000 | 0x00000001 | 0x00000002)
while not (D / "ab.done").exists():
    time.sleep(30)
