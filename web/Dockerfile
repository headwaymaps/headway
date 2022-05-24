FROM nginx

RUN apt-get update

RUN apt-get install -y curl

RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -

RUN apt-get install -y nodejs

RUN npm install --global yarn

RUN yarn global add @quasar/cli

COPY frontend /frontend

WORKDIR /frontend

RUN yarn cache clean

RUN rm -rf ./.node_modules

RUN rm -rf ./.quasar

RUN yarn install

RUN quasar build

RUN cp -r /frontend/dist/spa/* /usr/share/nginx/html

COPY nginx.conf /etc/nginx/nginx.conf

COPY .base_url /etc/nginx/

COPY bbox.txt /usr/share/nginx/html/bbox.txt

RUN bash -c 'sed -i "s^__HEADWAY_BASE_URL__^$(</etc/nginx/.base_url)^" /etc/nginx/nginx.conf'
