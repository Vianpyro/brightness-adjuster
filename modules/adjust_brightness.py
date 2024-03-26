# -*- coding: utf-8 -*-
import bisect
from datetime import datetime

import screen_brightness_control as sbc


def find_last_passed_hour(spans: dict) -> str:
    """
    Finds the last passed hour based on the current time and brightness spans.

    Args:
        spans (dict): A dictionary containing brightness spans with time as keys.

    Returns:
        str: The last passed hour in 24-hour format (HH:MM).
    """
    current_time = datetime.now().time()
    sorted_times = sorted(spans.keys())

    index = bisect.bisect_left(sorted_times, current_time.strftime("%H:%M"))

    return sorted_times[index - 1] if index > 0 else sorted_times[0]


def update_brightness(spans: dict, current_time: str) -> None:
    """
    Updates the screen brightness based on the provided brightness span for the current time.

    Args:
        spans (dict): A dictionary containing brightness spans with time as keys.
        current_time (str): The current time in 24-hour format (HH:MM).

    Returns:
        None
    """
    brightness = spans[current_time]
    sbc.fade_brightness(brightness, increment=1)
