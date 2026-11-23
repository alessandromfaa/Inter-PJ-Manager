FROM node:22-bookworm-slim AS build
WORKDIR /src
COPY package*.json ./
RUN npm ci --legacy-peer-deps
COPY . .
RUN npm run build

FROM nginxinc/nginx-unprivileged:stable-alpine
COPY nginx.conf /etc/nginx/nginx.conf
COPY --from=build /src/dist /usr/share/nginx/html
ENTRYPOINT ["nginx","-g","daemon off;"]
