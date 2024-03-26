# -*- coding: utf-8 -*-
import requests


def get_astronomy(API_KEY) -> dict:
    url = f"https://api.ipgeolocation.io/astronomy?apiKey={API_KEY}"
    response = requests.get(url)
    data = response.json()
    return data
