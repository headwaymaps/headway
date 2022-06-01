FROM python:3

RUN pip install requests

WORKDIR /gtfs

RUN mkdir /gtfs_feeds

COPY download_gtfs_feeds.py /gtfs/

ARG HEADWAY_BBOX

ENV HEADWAY_BBOX=${HEADWAY_BBOX}

RUN python /gtfs/download_gtfs_feeds.py

RUN bash -c "cd /gtfs_feeds && ls *.zip | tar -cf gtfs.tar --files-from -"
