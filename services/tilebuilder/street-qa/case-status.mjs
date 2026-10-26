// Keeps the street QA page's verdicts, notes and added cases in cases.json, for Claude to read.
// Usage: node case-status.mjs [cases.json]
import { createServer } from 'node:http';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
const file = process.argv[2] ?? new URL('cases.json', import.meta.url).pathname;
const read = () => (existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : {});
createServer((req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, PUT, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  if (req.method === 'OPTIONS') return res.end();
  if (req.method === 'GET') return res.end(JSON.stringify(read()));
  let body = '';
  req.on('data', (chunk) => { body += chunk; });
  req.on('end', () => {
    const { title, ...fields } = JSON.parse(body);
    const all = read();
    all[title] = { ...all[title], ...fields, updated: new Date().toISOString() };
    writeFileSync(file, JSON.stringify(all, null, 2) + '\n');
    res.end(JSON.stringify(all));
  });
}).listen(8091, '127.0.0.1');
