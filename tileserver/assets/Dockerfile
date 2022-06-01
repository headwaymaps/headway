FROM node:12

RUN npm install fontnik

COPY build_glyphs.js build_sprites.js package.json package-lock.json *.ttf /app/

WORKDIR /app

RUN npm install

RUN mkdir -p /app/sprite/

COPY sprites/*.svg /app/sprite/

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

RUN node build_sprites /output/sprite /app/sprite
RUN node build_sprites --retina /output/sprite@2x /app/sprite

WORKDIR /output

RUN tar -cf fonts.tar "Noto Sans Bold" "Noto Sans Italic" "Noto Sans Regular"
RUN tar -cf sprite.tar sprite.json sprite.png sprite@2x.json sprite@2x.png
