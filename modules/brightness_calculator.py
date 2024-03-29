"""
Module: brightness_calculator

This module provides functions for calculating brightness levels
based on sunrise, solar noon, and specified brightness range.

Functions:
    validate_brightness_values(min_brightness, max_brightness):
        Validates the provided minimum and maximum brightness values.
    calculate_brightness_duration(sunrise, solar_noon, spans_count):
        Calculates the duration of each brightness span.
    calculate_brightness(min_brightness, max_brightness, spans_duration, sunrise):
        Calculates the brightness level for each time span between sunrise and solar noon.
    brightness_spans_calculator(sunrise, solar_noon, min_brightness, max_brightness):
        Calculates brightness spans based on sunrise, solar noon, and brightness range.
"""

from datetime import datetime, timedelta

TIME_FORMAT = "%H:%M"


def validate_brightness_values(min_brightness, max_brightness) -> None:
    """
    Validates the provided minimum and maximum brightness values.

    Args:
        min_brightness (int): Minimum brightness value.
        max_brightness (int): Maximum brightness value.

    Raises:
        ValueError: If the brightness values are not within the valid range
            or if min_brightness >= max_brightness.
    """
    if min_brightness < 0 or max_brightness > 100:
        raise ValueError("Brightness values must be between 0 and 100")

    if min_brightness >= max_brightness:
        raise ValueError("Min brightness must be less than max brightness")


def calculate_brightness_duration(sunrise, solar_noon, spans_count) -> float:
    """
    Calculates the duration of each brightness span.

    Args:
        sunrise (str): The time of sunrise in 24-hour format (HH:MM).
        solar_noon (str): The time of solar noon in 24-hour format (HH:MM).
        spans_count (float): Number of brightness spans.

    Returns:
        float: Duration of each brightness span in minutes.
    """
    noon_time = datetime.strptime(solar_noon, TIME_FORMAT)
    sunrise_time = datetime.strptime(sunrise, TIME_FORMAT)

    spans_duration = noon_time - sunrise_time
    spans_duration_minutes = float(spans_duration.total_seconds() / 60 / spans_count)

    return spans_duration_minutes


def calculate_brightness(
    min_brightness, max_brightness, spans_duration, sunrise
) -> dict:
    """
    Calculates the brightness level for each time span between sunrise and solar noon.

    Args:
        min_brightness (int): Minimum brightness value.
        max_brightness (int): Maximum brightness value.
        spans_duration (int): Duration of each brightness span in minutes.
        sunrise (str): The time of sunrise in 24-hour format (HH:MM).

    Returns:
        dict: A dictionary mapping time (in 24-hour format) to brightness level.
    """
    spans = {}

    for i in range((max_brightness - min_brightness) * 2 + 1):
        delta = timedelta(minutes=spans_duration * i)
        brightness = int(
            min_brightness
            + (max_brightness - min_brightness) * (1 - abs(i % (100 * 2) - 100) / 100)
        )
        time = datetime.strftime(sunrise + delta, TIME_FORMAT)
        spans[time] = brightness

    return spans


def brightness_spans_calculator(
    sunrise, solar_noon, min_brightness, max_brightness
) -> dict:
    """
    Calculates brightness spans based on sunrise, solar noon, and brightness range.

    Args:
        sunrise (str): The time of sunrise in 24-hour format (HH:MM).
        solar_noon (str): The time of solar noon in 24-hour format (HH:MM).
        min_brightness (int): Minimum brightness value.
        max_brightness (int): Maximum brightness value.

    Returns:
        dict: A dictionary mapping time (in 24-hour format) to brightness level.
    """
    validate_brightness_values(min_brightness, max_brightness)

    spans_count = max_brightness - min_brightness
    spans_duration = calculate_brightness_duration(sunrise, solar_noon, spans_count)
    sunrise = datetime.strptime(sunrise, TIME_FORMAT)

    return calculate_brightness(min_brightness, max_brightness, spans_duration, sunrise)
