FROM python:3

RUN pip install requests

WORKDIR /gtfs

RUN mkdir /gtfs_feeds

COPY download_gtfs_feeds.py /gtfs/
COPY gtfs_feeds.gen.csv /gtfs/gtfs_feeds.csv

RUN python /gtfs/download_gtfs_feeds.py

RUN bash -c "cd /gtfs_feeds && ls *.zip | tar -cf gtfs.tar --files-from -"
