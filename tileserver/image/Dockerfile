FROM maptiler/tileserver-gl

COPY configure_run.sh config.json.template /app/

ENTRYPOINT [ "/bin/bash" ]

CMD [ "/app/configure_run.sh" ]
