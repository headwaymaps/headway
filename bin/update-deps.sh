set -ex

(cd services/frontend/www-app && yarn upgrade) \
    && (cd services/pelias/generate_config && yarn upgrade) \
    && (cd services/tileserver/assets && npm update) \
    && (cd services/travelmux && cargo update) \
    && (cd services/gtfs/gtfout && cargo update) \
    && (cd builds/planet/assemble-planet-pbf && cargo update)

