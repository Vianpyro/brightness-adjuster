# -*- coding: utf-8 -*-
from datetime import datetime
import bisect
import screen_brightness_control as sbc

def find_last_passed_hour(spans: dict) -> str:
    current_time = datetime.now().time()
    sorted_times = sorted(spans.keys())

    index = bisect.bisect_left(sorted_times, current_time.strftime('%H:%M'))

    return sorted_times[index - 1] if index > 0 else sorted_times[0]

def update_brightness(spans: dict, current_time: str) -> None:
    brightness = spans[current_time]

    sbc.fade_brightness(brightness, increment=1)
