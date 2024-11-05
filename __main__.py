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


def load_and_validate_env():
    """
    Load environment variables from a .env file and validate required variables.
    
    Returns:
        str: The API_KEY from the .env file.
        
    Raises:
        FileNotFoundError: If the .env file does not exist.
        ValueError: If API_KEY is not found in the .env file.
    """
    env_path = os.path.join(os.path.dirname(__file__), ".env")

    if not os.path.exists(env_path):
        raise FileNotFoundError("No '.env' file found. Please create a '.env' file with the API_KEY.")

    load_dotenv(env_path)

    api_key = os.getenv("API_KEY")
    if not api_key:
        raise ValueError("No API_KEY found in '.env' file. Please add the API_KEY to the '.env' file.")

    return api_key

def main():
    """
    Main function to calculate and adjust brightness based on sunrise and sunset times.
    """
    API_KEY = load_and_validate_env()
    astronomy = adf.get_astronomy(API_KEY)

    sunrise = astronomy["sunrise"]
    noon = astronomy["solar_noon"]

    spans = bca.brightness_spans_calculator(sunrise, noon, 0, 100)

    while True:
        bco.update_brightness(spans, bco.find_last_passed_hour(spans))
        time.sleep(60 * 3)


if __name__ == "__main__":
    main()
