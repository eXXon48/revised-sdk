// Stand-in for the store api in the preview container (a fake unit has no server).
// The daemon records every unsigned install with `POST <api>/devmode-event` and refuses the
// install unless the server confirms; this answers that call with 200 and everything else with 404.
require("http").createServer(function (req, res) {
  req.resume();
  req.on("end", function () {
    var ok = req.method === "POST" && /\/devmode-event$/.test(req.url.split("?")[0]);
    res.writeHead(ok ? 200 : 404, { "Content-Type": "application/json" });
    res.end(ok ? "{}" : '{"error":"not available in the preview"}');
  });
}).listen(8770, "127.0.0.1");
