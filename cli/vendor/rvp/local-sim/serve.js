// DEV ONLY static server for the local HMI sim (runs in the rvp-sim pod, node:22-alpine):
//   /            -> $ROOT     (sim root: /jci/gui/index.html, /data, /data_persist)
//   /catalog/... -> $CATALOG  (catalog-dev: catalog.json, .minisig, pkgs/*.rvp)
// Published on the host as 127.0.0.1:8750 only; the daemon reaches it inside the pod.
var http = require("http"), fs = require("fs"), path = require("path");
var ROOT = path.resolve(process.env.ROOT || path.join(__dirname, "root"));
var CATALOG = path.resolve(process.env.CATALOG || path.join(__dirname, "..", "catalog-dev"));
var TYPES = { ".html": "text/html", ".js": "application/javascript", ".json": "application/json", ".png": "image/png", ".css": "text/css" };
http.createServer(function (req, res) {
  var u = decodeURIComponent(req.url.split("?")[0]), base = ROOT;
  if (u.indexOf("/catalog/") === 0) { base = CATALOG; u = u.substring(8); }
  var p = path.normalize(path.join(base, u));
  if (p.indexOf(base) !== 0) { res.writeHead(403); return res.end(); }
  fs.readFile(p, function (err, b) {
    if (err) { res.writeHead(404); return res.end("404"); }
    res.writeHead(200, { "Content-Type": TYPES[path.extname(p)] || "application/octet-stream", "Cache-Control": "no-store" });
    res.end(b);
  });
}).listen(8750, "0.0.0.0", function () { console.log("sim http :8750 root=" + ROOT + " catalog=" + CATALOG); });
