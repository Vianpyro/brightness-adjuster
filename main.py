# -*- coding: ascii -*-
from datetime import date, datetime, timedelta
import bisect
import json
import os
import subprocess
import threading

# Install the required libraries
subprocess.Popen('pip3 install -r requirements.txt')

# Import the required libraries
from dotenv import load_dotenv
from PIL import Image
import pystray
import requests
import schedule
import screen_brightness_control as sbc

# Retrieve the .env file data
load_dotenv()
API_KEY = os.getenv('IPGEOLOCATION.IO_API_KEY')


def open_file(path) -> None:
    if os.path.exists(path):
        os.startfile(path)
    else:
        print(f'{path} does not exist.')


class Brightess_Ajuster:
    def __init__(self, **kwargs) -> None:
        """
        Initializes the Brightness_Ajuster object with the current brightness of the device's display, \
            the current date, and the minimum and maximum brightness values the user wants.
        An astronomical data API is called to calculate the brightness values based on \
            the device's location and the current date.
        """
        self.current_brightness = sbc.get_brightness()
        self.today = date.today()
        self.time_format = '%H:%M'

        self.min_brightness = kwargs['min_brightness'] if 'min_brightness' in kwargs else 0
        self.max_brightness = kwargs['max_brightness'] if 'max_brightness' in kwargs else 100
        self.saved_date = None

        if 'date' in kwargs and (saved_date := datetime.strptime(kwargs['date'], '%Y-%m-%d').date()):
            self.saved_date = saved_date
        
        if self.saved_date == self.today and 'brightness_spans' in kwargs:
            if len(kwargs['brightness_spans']) >= 3:
                self.brightness_spans = kwargs['brightness_spans']
        else:
            self.get_astronomy()
            self.brightness_spans = self.brightness_spans_calculator()

    def get_astronomy(self):
        """
        Uses an API to get the astronomical data for the device's location, including sunrise, solar noon, and sunset times.
        """
        self.astronomy = requests.get(f'https://api.ipgeolocation.io/astronomy?apiKey={API_KEY}').json()

    def brightness_spans_calculator(self) -> list:
        """
        Calculates the brightness values for the device's display at different times of day based on the astronomical data.
        The calculated values are returned as a dictionary.
        """
        morn_time = datetime.strptime(self.astronomy['sunrise'], self.time_format)
        noon_time = datetime.strptime(self.astronomy['solar_noon'], self.time_format)
        even_time = datetime.strptime(self.astronomy['sunset'], self.time_format)

        morning_brightness = {
            time_iter.strftime(self.time_format): round(self.min_brightness + (self.max_brightness - self.min_brightness) * ((time_iter - morn_time).total_seconds() / (noon_time - morn_time).total_seconds())) 
                for time_iter in (morn_time + timedelta(minutes=i) \
                for i in range((noon_time - morn_time).seconds // 60 + 1)) \
                if round(self.min_brightness + (self.max_brightness - self.min_brightness) * ((time_iter - morn_time).total_seconds() / (noon_time - morn_time).total_seconds())) != round(
                    self.min_brightness + (self.max_brightness - self.min_brightness) * ((time_iter + timedelta(minutes=1) - morn_time).total_seconds() / (noon_time - morn_time).total_seconds()))
        }
        afternoon_brightness = {
            time_iter.strftime(self.time_format): round(self.max_brightness + (self.min_brightness - self.max_brightness) * ((time_iter - noon_time).total_seconds() / (even_time - noon_time).total_seconds())) 
                for time_iter in (noon_time + timedelta(minutes=i) \
                for i in range((even_time - noon_time).seconds // 60 + 1)) \
                if round(self.max_brightness + (self.min_brightness - self.max_brightness) * ((time_iter - noon_time).total_seconds() / (even_time - noon_time).total_seconds())) != round(
                    self.max_brightness + (self.min_brightness - self.max_brightness) * ((time_iter + timedelta(minutes=1) - noon_time).total_seconds() / (even_time - noon_time).total_seconds()))
        }
        
        brightness_spans = morning_brightness | afternoon_brightness
        if len(brightness_spans) < 3:
            return {
                self.astronomy['sunrise']: self.min_brightness,
                self.astronomy['solar_noon']: self.max_brightness,
                self.astronomy['sunset']: self.min_brightness,
            }

        last_times = list(afternoon_brightness.keys())[-2:]
        time_spans = [(datetime.strptime(last_times[i+1], self.time_format) - datetime.strptime(last_times[i], self.time_format)).total_seconds() / 60 for i in range(len(last_times) - 1)]

        last_ray_of_sunshine = {
            datetime.strftime(datetime.strptime(last_times[1], self.time_format) + timedelta(minutes=sum(time_spans) / len(time_spans)), self.time_format): self.min_brightness
        }

        return brightness_spans | last_ray_of_sunshine
        
    def adjust_brightness(self) -> None:
        """
        Checks the current time against the brightness values dictionary and adjusts the device's display brightness accordingly.
        """
        current_time = datetime.now().time()
        sorted_times = sorted(self.brightness_spans.keys())

        index = bisect.bisect_left(sorted_times, current_time.strftime('%H:%M'))
        latest_time_passed = None

        if index > 0:
            latest_time_passed = sorted_times[index-1]

        log_time = datetime.strftime(datetime.now(), "%H:%M:%S")

        if latest_time_passed is not None:
            print(f'[{log_time}] The latest time that has already passed is {latest_time_passed}.')
            print(f'Brightness to: {self.brightness_spans[latest_time_passed]}')
            sbc.fade_brightness(self.brightness_spans[latest_time_passed], increment=1)
        else:
            print(f'[{log_time}] None of the times in the dictionary have already passed.')
            print(f'Brightness to: {self.min_brightness}')
            sbc.fade_brightness(self.min_brightness, increment=1)

    def check_date_accuracy(self) -> None:
        """
        Checks whether the current date has changed since the object was created and recalculates the brightness values if necessary.
        """
        d = date.today()

        if d != self.today:
            self.get_astronomy()
            self.brightness_spans = self.brightness_spans_calculator()
            self.today = date.today()
            print(f'Today is a new day: {self.today}')

    def get_brightess_ajuster_settings(self) -> dict:
        """
        Returns a dictionary containing the object's current settings, including \
            the minimum and maximum brightness values, the current date, and the calculated brightness values.
        """
        return {
            'min_brightness': self.min_brightness,
            'max_brightness': self.max_brightness,
            'date': str(self.today),
            "brightness_spans": self.brightness_spans
        }

class Tray_Icon:
    def __init__(self) -> None:
        self.running = True
        self.icon = pystray.Icon(
            name='Hello World!',
            icon=Image.open('tray_icon.png'),
            title='Loading...',
            menu=(
                pystray.MenuItem('Settings', lambda: open_file(os.path.join(os.getcwd(), 'save.json'))),
                pystray.Menu.SEPARATOR,
                pystray.MenuItem('Exit', self.exit)
            )
        )
        self.update_title()

    def exit(self):
        print('Exit clicked!')
        self.running = False
        self.icon.stop()

    def update_title(self):
        displays_brightness = sbc.get_brightness()

        if len(set(displays_brightness)) == 1:
            self.icon.title = f'Brightness: {displays_brightness[0]}%'
        else:
            self.icon.title = 'Brightness: {}'.format(', '.join(f'{x}%' for x in displays_brightness))

    def run(self):
        self.icon.run()


data = {}
if os.path.exists('save.json'):
    with open('save.json', 'r') as f:
        data = json.load(f)

tray = Tray_Icon()
icon_thread = threading.Thread(target=tray.run)
icon_thread.start()
twinkle = Brightess_Ajuster(**data)

schedule.every(90).seconds.do(twinkle.adjust_brightness)
schedule.every(1).minutes.do(tray.update_title)
schedule.every(1).hours.do(twinkle.check_date_accuracy)

while tray.running:
    try:
        schedule.run_pending()
    except Exception as e:
        print(e)

# Save the brightess ajuster settings
with open('save.json', 'w') as f:
    json.dump(twinkle.get_brightess_ajuster_settings(), f, indent=4)
