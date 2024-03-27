# -*- coding: utf-8 -*-
"""
Module: __main__

This module serves as the entry point for calculating and adjusting brightness based on sunrise and
    sunset times.

Functions:
    main():
        Main function to calculate and adjust brightness based on sunrise and sunset times.
"""

import os
import time
from dotenv import load_dotenv

import modules.astronomy_data_fetcher as adf
import modules.brightness_calculator as bca
import modules.brightness_controller as bco


def main():
    """
    Main function to calculate and adjust brightness based on sunrise and sunset times.
    """
    load_dotenv()

    API_KEY = os.getenv("API_KEY")

    astronomy = adf.get_astronomy(API_KEY)

    sunrise = astronomy["sunrise"]
    noon = astronomy["solar_noon"]

    spans = bca.brightness_spans_calculator(sunrise, noon, 0, 100)

    while True:
        bco.update_brightness(spans, bco.find_last_passed_hour(spans))
        time.sleep(60 * 3)


if __name__ == "__main__":
    main()
