FROM node:8

USER root

RUN npm install fontnik

COPY build_glyphs.js *.ttf /app/

WORKDIR /app

RUN mkdir /output

RUN useradd -ms /bin/bash fontnik

RUN chown fontnik /output

USER fontnik

RUN mkdir "/output/Noto Sans Bold"
RUN mkdir "/output/Noto Sans Italic"
RUN mkdir "/output/Noto Sans Regular"

RUN node build_glyphs NotoSans-Bold.ttf "/output/Noto Sans Bold"
RUN node build_glyphs NotoSans-Italic.ttf "/output/Noto Sans Italic"
RUN node build_glyphs NotoSans-Regular.ttf "/output/Noto Sans Regular"

WORKDIR /output

RUN tar -cf fonts.tar "Noto Sans Bold" "Noto Sans Italic" "Noto Sans Regular"
