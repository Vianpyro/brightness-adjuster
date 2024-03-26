# -*- coding: utf-8 -*-
from dotenv import load_dotenv
import os

import modules.brightness_spans as bs
import modules.get_astronomy as ga
import modules.adjust_brightness as ab


load_dotenv()

API_KEY = os.getenv("API_KEY")

if __name__ == "__main__":
    astronomy = ga.get_astronomy(API_KEY)

    sunrise = astronomy["sunrise"]
    noon = astronomy["sunset"]

    spans = bs.brightness_spans_calculator(sunrise, noon, 0, 100)

    ab.update_brightness(spans, ab.find_last_passed_hour(spans))
