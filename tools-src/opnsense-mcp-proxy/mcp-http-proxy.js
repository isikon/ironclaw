import http from 'http';
import { spawn } from 'child_process';
import { createInterface } from 'readline';

const PORT = process.env.PROXY_PORT || 3742;
const url = process.env.OPNSENSE_URL;
const key = process.env.OPNSENSE_API_KEY;
const secret = process.env.OPNSENSE_API_SECRET;

if (!url || !key || !secret) {
  console.error('Missing OPNSENSE_URL, OPNSENSE_API_KEY or OPNSENSE_API_SECRET');
  process.exit(1);
}

const args = [
  '/home/ironclaw/opnsense-mcp-server/index.js',
  '--url', url, '--api-key', key, '--api-secret', secret,
  '--no-verify-ssl', '--plugins'
];

const child = spawn('node', args, { stdio: ['pipe', 'pipe', 'inherit'] });
child.on('exit', (code) => { console.error(`MCP exited: ${code}`); process.exit(code || 1); });

const pending = new Map();
const rl = createInterface({ input: child.stdout });
rl.on('line', (line) => {
  if (!line.trim()) return;
  let msg; try { msg = JSON.parse(line); } catch { return; }
  if (msg.id == null) return;
  const p = pending.get(msg.id);
  if (p) { pending.delete(msg.id); p.resolve(msg); }
});

function callMcp(req) {
  return new Promise((resolve, reject) => {
    pending.set(req.id, { resolve, reject });
    child.stdin.write(JSON.stringify(req) + '\n');
    setTimeout(() => { if (pending.has(req.id)) { pending.delete(req.id); reject(new Error('Timeout')); } }, 30000);
  });
}

http.createServer(async (req, res) => {
  if (req.method !== 'POST') { res.writeHead(405); res.end(); return; }
  let body = '';
  req.on('data', c => body += c);
  req.on('end', async () => {
    let request; try { request = JSON.parse(body); } catch { res.writeHead(400); res.end(); return; }
    if (request.id == null) { child.stdin.write(JSON.stringify(request) + '\n'); res.writeHead(202); res.end(); return; }
    try {
      const response = await callMcp(request);
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(response));
    } catch (e) {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ jsonrpc: '2.0', id: request.id, error: { code: -32000, message: e.message } }));
    }
  });
}).listen(PORT, '127.0.0.1', () => console.log(`MCP proxy on http://127.0.0.1:${PORT}`));
