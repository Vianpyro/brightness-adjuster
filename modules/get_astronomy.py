# -*- coding: utf-8 -*-
import requests


def get_astronomy(api_key) -> dict:
    """
    Retrieves astronomy data using the provided API key.

    Args:
        api_key (str): The API key for accessing astronomy data.

    Returns:
        dict: Astronomy data retrieved from the API.
    """
    url = f"https://api.ipgeolocation.io/astronomy?apiKey={api_key}"
    response = requests.get(url)
    data = response.json()
    return data
