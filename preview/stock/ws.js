// Minimal RFC 6455 server for the preview stubs: text frames, ping/pong, close. No fragmentation, no deps.
var http = require("http"), crypto = require("crypto"), EventEmitter = require("events");
var GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

function frame(op, payload) {
  var n = payload.length, h;
  if (n < 126) h = Buffer.from([0x80 | op, n]);
  else if (n < 65536) { h = Buffer.alloc(4); h[0] = 0x80 | op; h[1] = 126; h.writeUInt16BE(n, 2); }
  else { h = Buffer.alloc(10); h[0] = 0x80 | op; h[1] = 127; h.writeUInt32BE(0, 2); h.writeUInt32BE(n, 6); }
  return Buffer.concat([h, payload]);
}

function Conn(sock) {
  EventEmitter.call(this);
  var self = this, buf = Buffer.alloc(0);
  this.sock = sock;
  sock.on("data", function (d) {
    buf = Buffer.concat([buf, d]);
    for (;;) {
      if (buf.length < 2) return;
      var op = buf[0] & 0x0f, masked = buf[1] & 0x80, n = buf[1] & 0x7f, off = 2;
      if (n === 126) { if (buf.length < 4) return; n = buf.readUInt16BE(2); off = 4; }
      else if (n === 127) { if (buf.length < 10) return; n = buf.readUInt32BE(6); off = 10; }
      var mk = masked ? buf.slice(off, off + 4) : null;
      if (masked) off += 4;
      if (buf.length < off + n) return;
      var p = Buffer.from(buf.slice(off, off + n));
      if (mk) for (var i = 0; i < n; i++) p[i] ^= mk[i % 4];
      buf = buf.slice(off + n);
      if (op === 1) {
        var s = p.toString();
        try { self.emit("msg", JSON.parse(s)); } catch (e) { self.emit("raw", s); }
      } else if (op === 8) { sock.end(frame(8, Buffer.alloc(0))); }
      else if (op === 9) { sock.write(frame(10, p)); }
    }
  });
  sock.on("close", function () { self.emit("close"); });
  sock.on("error", function () {});
}
Conn.prototype = Object.create(EventEmitter.prototype);
Conn.prototype.send = function (obj) {
  if (!this.sock.destroyed) this.sock.write(frame(1, Buffer.from(typeof obj === "string" ? obj : JSON.stringify(obj))));
};

// listen(port, protocol, onConn) -> http.Server; binds 127.0.0.1
exports.listen = function (port, protocol, onConn) {
  var srv = http.createServer(function (req, res) { res.writeHead(426); res.end(); });
  srv.on("upgrade", function (req, sock) {
    var key = req.headers["sec-websocket-key"];
    var accept = crypto.createHash("sha1").update(key + GUID).digest("base64");
    sock.write("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" +
      "Sec-WebSocket-Accept: " + accept + "\r\n" +
      (req.headers["sec-websocket-protocol"] ? "Sec-WebSocket-Protocol: " + protocol + "\r\n" : "") + "\r\n");
    onConn(new Conn(sock), req);
  });
  srv.listen(port, "127.0.0.1");
  return srv;
};
