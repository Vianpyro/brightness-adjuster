# -*- coding: utf-8 -*-
"""
Module: __main__

This module serves as the entry point for calculating and adjusting brightness based on sunrise and sunset times.

Functions:
    main():
        Main function to calculate and adjust brightness based on sunrise and sunset times.
"""

import os
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
    noon = astronomy["sunset"]

    spans = bca.brightness_spans_calculator(sunrise, noon, 0, 100)

    bco.update_brightness(spans, bco.find_last_passed_hour(spans))


if __name__ == "__main__":
    main()
