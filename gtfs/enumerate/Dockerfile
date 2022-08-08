FROM python:3

RUN pip install requests

WORKDIR /gtfs

RUN mkdir /gtfs_feeds

COPY enumerate_gtfs_feeds.py /gtfs/

ARG HEADWAY_BBOX

ENV HEADWAY_BBOX=${HEADWAY_BBOX}

RUN python /gtfs/enumerate_gtfs_feeds.py
