# -*- coding: utf-8 -*-
import os
from dotenv import load_dotenv

import modules.brightness_spans as bs
import modules.get_astronomy as ga
import modules.adjust_brightness as ab


def main():
    """
    Main function to calculate and adjust brightness based on sunrise and sunset times.
    """
    load_dotenv()

    API_KEY = os.getenv("API_KEY")

    astronomy = ga.get_astronomy(API_KEY)

    sunrise = astronomy["sunrise"]
    noon = astronomy["sunset"]

    spans = bs.brightness_spans_calculator(sunrise, noon, 0, 100)

    ab.update_brightness(spans, ab.find_last_passed_hour(spans))


if __name__ == "__main__":
    main()
