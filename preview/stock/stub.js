// Stub backend for the stock GUI in `revised-sdk preview up --stock`: the three WebSocket servers
// the GUI connects to (MMUI :2700, AppSDK :2800, DBAPI :2766) and a control port.
//
// fixtures/*.json hold an array of rules:
//   { "after": "connect" | "<eventId>" | "ctx:<uiaId>/<ctxtId>", "send": [<MMUI frame>, ...] }
//   { "dbapi": "<api>/<methodName>", "params": {...} }
// "ctx:" rules fire 1.5 s after a screen switch (data frames need the screen template up).
// A "dbapi" rule answers a GUI method call with a methodResponse that carries the request's
// api, methodName and reqId and the rule's params.
//
// ctxtChg, focusStack and transition frames end with one extra character, the sequence counter
// the GUI reads: a ctxtChg starts a new value, the focusStack and transition after it reuse it.
//
// Control (127.0.0.1:8761):
//   POST /ctx/<uiaId>/<ctxtId>   switch screens (ctxtChg + focusStack + transition)
//   POST /send                   body = one MMUI frame or an array of frames
// Every frame in and out is one JSON line in $STUB_LOG (default /tmp/preview-stub.log).
var fs = require("fs"), path = require("path"), http = require("http"), ws = require("./ws");
var LOG = fs.createWriteStream(process.env.STUB_LOG || "/tmp/preview-stub.log", { flags: "w" });
function log(dir, sock, o) { LOG.write(JSON.stringify({ t: Date.now(), dir: dir, sock: sock, m: o }) + "\n"); }

var rules = [];
var FIX = path.join(__dirname, "fixtures");
fs.readdirSync(FIX).sort().forEach(function (f) {
  if (/\.json$/.test(f)) rules = rules.concat(JSON.parse(fs.readFileSync(path.join(FIX, f), "utf8")));
});

var mmui = null, seq = 0, ctxSeq = 100;
var SEQUENCED = { ctxtChg: 1, focusStack: 1, transition: 1 };

function sendMmui(m) {
  if (!mmui) return log("drop", "mmui", m);
  if (m.msgType === "ctxtChg") {
    seq = seq % 250 + 1;
    if (m.contextSeq === undefined) m.contextSeq = ++ctxSeq;
    if (m.params === undefined) m.params = {};
  }
  var s = JSON.stringify(m);
  if (SEQUENCED[m.msgType]) s += String.fromCharCode(seq);
  log("out", "mmui", m);
  mmui.send(s);
}

function fire(trigger) {
  rules.forEach(function (r) {
    if (r.after === trigger) r.send.forEach(function (m) { sendMmui(JSON.parse(JSON.stringify(m))); });
  });
}

ws.listen(2700, "ihu-guiifm-protocol", function (c) {
  mmui = c; seq = 0; log("open", "mmui", null);
  c.on("msg", function (o) { log("in", "mmui", o); if (o.msgType === "event") fire(o.eventId); });
  c.on("raw", function (s) { log("in-raw", "mmui", s); });
  c.on("close", function () { log("close", "mmui", null); if (mmui === c) mmui = null; });
  fire("connect");
});
["appsdk:2800:ihu-appsdk-protocol", "dbapi:2766:ihu-dbapi-protocol"].forEach(function (d) {
  var p = d.split(":");
  ws.listen(+p[1], p[2], function (c) {
    log("open", p[0], null);
    c.on("msg", function (o) {
      log("in", p[0], o);
      var mc = o.msgContent;
      if (p[0] !== "dbapi" || o.msgType !== "method" || !mc) return;
      rules.forEach(function (r) {
        if (r.dbapi !== mc.api + "/" + mc.methodName) return;
        var out = { msgType: "methodResponse", msgContent: { api: mc.api, methodName: mc.methodName, reqId: mc.reqId, params: r.params } };
        log("out", "dbapi", out); c.send(out);
      });
    });
    c.on("raw", function (s) { log("in-raw", p[0], s); });
    c.on("close", function () { log("close", p[0], null); });
  });
});

http.createServer(function (req, res) {
  var body = "";
  req.on("data", function (d) { body += d; });
  req.on("end", function () {
    var m = req.url.match(/^\/ctx\/(\w+)\/(\w+)$/);
    try {
      if (m) {
        sendMmui({ msgType: "ctxtChg", uiaId: m[1], ctxtId: m[2] });
        sendMmui({ msgType: "focusStack", appIdList: [{ id: m[1] }, { id: "system" }] });
        sendMmui({ msgType: "transition", enabled: false });
        setTimeout(function () { fire("ctx:" + m[1] + "/" + m[2]); }, 1500);
      } else if (req.url === "/send") {
        [].concat(JSON.parse(body)).forEach(sendMmui);
      } else { res.writeHead(404); return res.end(); }
      res.end(mmui ? "ok\n" : "no mmui\n");
    } catch (e) { res.writeHead(400); res.end(String(e) + "\n"); }
  });
}).listen(8761, "127.0.0.1");
console.log("[preview] stock stubs up: mmui 2700, appsdk 2800, dbapi 2766, control 8761, rules " + rules.length);
