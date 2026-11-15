// Minimal WebSocket client for the preview container (node 18 has no global WebSocket).
// Usage: node wsclient.js <port> '<json to send>' '<substring of the reply frame to wait for>'
// Prints the first reply frame containing the substring, exit 0; exit 1 after 20 s.
var net = require("net"), crypto = require("crypto");
var port = +process.argv[2], msg = process.argv[3], want = process.argv[4];
var s = net.connect(port, "127.0.0.1"), buf = Buffer.alloc(0), up = false;
function done(code, text) { if (text) console.log(text); process.exit(code); }
setTimeout(function () { done(1, "timeout waiting for " + want); }, 20000);
s.on("error", function (e) { done(1, String(e)); });
s.on("connect", function () {
  s.write("GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" +
    "Sec-WebSocket-Key: " + crypto.randomBytes(16).toString("base64") + "\r\nSec-WebSocket-Version: 13\r\n\r\n");
});
function send(text) {
  var p = Buffer.from(text), mask = crypto.randomBytes(4), n = p.length, h, i;
  h = n < 126 ? Buffer.from([0x81, 0x80 | n]) : Buffer.from([0x81, 0xfe, n >> 8, n & 255]);
  for (i = 0; i < n; i++) p[i] ^= mask[i & 3];
  s.write(Buffer.concat([h, mask, p]));
}
s.on("data", function (d) {
  buf = Buffer.concat([buf, d]);
  var i;
  if (!up) {
    i = buf.indexOf("\r\n\r\n");
    if (i < 0) return;
    up = true; buf = buf.slice(i + 4); send(msg);
  }
  for (;;) {   // server frames are unmasked text
    if (buf.length < 2) return;
    var n = buf[1] & 127, o = 2;
    if (n === 126) { if (buf.length < 4) return; n = buf.readUInt16BE(2); o = 4; }
    else if (n === 127) { if (buf.length < 10) return; n = Number(buf.readBigUInt64BE(2)); o = 10; }
    if (buf.length < o + n) return;
    var text = buf.slice(o, o + n).toString("utf8");
    buf = buf.slice(o + n);
    if (text.indexOf(want) >= 0) done(0, text);
  }
});
