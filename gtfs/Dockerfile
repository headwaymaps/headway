FROM python:3

RUN pip install requests

WORKDIR /gtfs

COPY bboxes.csv download_gtfs_feeds.py /gtfs/

ARG CITY_NAME

ENV CITY_NAME=${CITY_NAME}

RUN mkdir /gtfs_feeds

RUN python /gtfs/download_gtfs_feeds.py

RUN bash -c "cd /gtfs_feeds && ls *.zip | tar -cf gtfs.tar --files-from -"