// Manual WS test client for the ws-logger. Fires one of each event shape the
// test app emits, then closes. Node v22+ (global WebSocket, no deps).
//   node ws-client.js [ws://host:port]
const url = process.argv[2] || 'ws://127.0.0.1:2777';
const ws = new WebSocket(url);
const events = [
  { control: 'button', action: 'press', value: 'primary', ts: Date.now() },
  { control: 'checkbox', action: 'toggle', value: { id: 'touch_in_motion', checked: false }, ts: Date.now() },
  { control: 'radio', action: 'select', value: 'developer', ts: Date.now() },
  { control: 'accordion', action: 'expand', value: 'daemon_config', ts: Date.now() },
  { control: 'tab', action: 'switch', value: 'nav', ts: Date.now() },
  { control: 'back', action: 'press', value: null, ts: Date.now() },
];
ws.addEventListener('open', () => {
  for (const e of events) ws.send(JSON.stringify(e));
  setTimeout(() => ws.close(), 300);
});
ws.addEventListener('error', (e) => { console.error('WS error:', e.message || e); process.exit(1); });
ws.addEventListener('close', () => { console.log('sent ' + events.length + ' events to ' + url + ', closed'); process.exit(0); });
setTimeout(() => { console.error('timeout — no connection to ' + url); process.exit(2); }, 4000);
