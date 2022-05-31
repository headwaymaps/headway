FROM python:3

RUN pip install requests

WORKDIR /gtfs

COPY bboxes.csv download_gtfs_feeds.py /gtfs/

ARG HEADWAY_AREA

ENV HEADWAY_AREA=${HEADWAY_AREA}

RUN mkdir /gtfs_feeds

RUN python /gtfs/download_gtfs_feeds.py

RUN bash -c "cd /gtfs_feeds && ls *.zip | tar -cf gtfs.tar --files-from -"
