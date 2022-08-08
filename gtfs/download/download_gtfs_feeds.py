import os
from urllib.request import urlopen
import requests
import csv

with open('/gtfs/gtfs_feeds.csv', 'r') as f:
  reader = csv.reader(f.readlines(), delimiter=',', quotechar='"')
  for line in reader:
    print("Downloading feed for", line[0].strip())
    with open('/gtfs_feeds/' + line[1].strip() + '.gtfs.zip', 'wb') as f:
      f.write(requests.get(line[2].strip()).content)
