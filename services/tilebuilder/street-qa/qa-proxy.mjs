// Serves tiles from a local martin (:8098) and everything else from the compose stack (:8080), on :8099.
import http from 'node:http';
http.createServer((req, res) => {
  const tiles = req.url.startsWith('/tileserver/');
  const options = tiles
    ? { port: 8098, path: req.url.replace(/^\/tileserver/, '') }
    : { port: 8080, path: req.url };
  const upstream = http.request({ host: '127.0.0.1', method: req.method, headers: req.headers, ...options }, (up) => {
    res.writeHead(up.statusCode, up.headers);
    up.pipe(res);
  });
  upstream.on('error', () => { res.writeHead(502); res.end(); });
  req.pipe(upstream);
}).listen(8099, '127.0.0.1');
