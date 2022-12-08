#!/bin/bash

function elastic_status(){
  curl \
    --output /dev/null \
    --silent \
    --write-out "%{http_code}" \
    "http://pelias-elasticsearch:9200/_cluster/health?wait_for_status=yellow&timeout=1s" \
      || true;
}

echo 'waiting for elasticsearch service to come up';
retry_count=600

i=1
while [[ "$i" -le "$retry_count" ]]; do
if [[ $(elastic_status) -eq 200 ]]; then
    echo "Elasticsearch up!"
    exit 0
elif [[ $(elastic_status) -eq 408 ]]; then
    # 408 indicates the server is up but not yet yellow status
    printf ":"
else
    printf "."
fi
sleep 1
i=$(($i + 1))
done

echo -e "\n"
echo "Elasticsearch did not come up, check configuration"
exit 1
