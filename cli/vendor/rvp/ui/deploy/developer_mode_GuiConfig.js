/* Revised Store - injector  (ES5 / Opera-Presto safe)
 * ----------------------------------------------------------------------------
 * Stock index.html loads  ../../data/integration/developer_mode_GuiConfig.js
 * if it exists. That is this file: the outward file of package `cmu-re-store`
 * (installed by the Revised daemon, the only file the package puts outside its jail).
 * It is removed by uninstalling the package and by a tripwire restore.
 *
 * What it does, all client-side, no native cooperation:
 *   1. loads the HMI SDK (sdk.js) from the package jail
 *        /data_persist/cmu-re/pkg/cmu-re-store/ui/sdk.js
 *      The SDK owns the menu rows, the overlay and the drag scroll; the store app
 *      (ui/controls.js) registers itself on it as "cmu-re-store/store".
 *   2. keeps the WebSocket to the Revised daemon (ws://127.0.0.1:2777) and exposes
 *      it to the store shim as window.__rvpDaemon = {send, onFrame, last}.
 *   3. if package cmu-re-dev-bridge is installed, loads its ui/bridge.js (remote eval,
 *      taps, knob keys) and hands it every non-JSON daemon line; else such lines are dropped.
 */
(function () {
  "use strict";

  var DAEMON_URL = "ws://127.0.0.1:2777";
  var TAG = "[revised-store]";
  function logMsg(m) {
    try { if (window.log && log.info) log.info(TAG + " " + m); } catch (e) {}
    try { if (window.console) console.log(TAG, m); } catch (e2) {}
  }

  // ---- WebSocket to the daemon (lazy, auto-reconnect) ---------------------
  var ws = null, wsErrLogged = false;
  var catFrame = null;   // last daemon `catalog` frame, replayed into a freshly opened overlay
  var subs = [];         // daemon-frame subscribers (the store shim)
  // keep the link up from boot so the daemon frames reach us
  setInterval(function () { ensureWs(); }, 5000);
  function ensureWs() {
    if (ws && (ws.readyState === 0 || ws.readyState === 1)) return;
    try {
      ws = new WebSocket(DAEMON_URL);
      ws.onopen = function () {
        logMsg("ws connected " + DAEMON_URL); wsErrLogged = false;
        // one-shot env probe: report the live Presto UA to the daemon log
        try { ws.send(JSON.stringify({ control: "env", action: "ua", value: navigator.userAgent, ts: Date.now() })); } catch (e) {}
      };
      ws.onmessage = function (m) {
        if (typeof m.data !== "string") return;
        // store frames (JSON): cache the catalog, hand every frame to the open renderer
        if (m.data.charAt(0) === "{") {
          var f = null;
          try { f = JSON.parse(m.data); } catch (e) { logMsg("bad frame: " + e); return; }
          if (f && f.t === "catalog") catFrame = f;
          toRenderer(f);
          return;
        }
        // any other line comes from the daemon's ctl FIFO: only the dev bridge handles it
        devLine(m.data);
      };
      ws.onclose = function () {
        ws = null;
        // link lost: drop the cached catalog -> store shows "Store service offline"
        if (catFrame) { catFrame = null; toRenderer({ t: "offline" }); }
      };
      ws.onerror = function () { if (!wsErrLogged) logMsg("ws error " + DAEMON_URL); wsErrLogged = true; };
    } catch (e) { logMsg("ws ctor failed: " + e); }
  }
  // ---- daemon link for the store shim (private to the store, not part of the SDK) ----
  function toRenderer(f) {
    var l = subs.slice(0), i;
    for (i = 0; i < l.length; i++) { try { l[i](f); } catch (e) { logMsg("onFrame subscriber failed: " + e); } }
  }
  function sendEvent(evt) {
    ensureWs();
    var json = JSON.stringify(evt);
    if (ws && ws.readyState === 1) ws.send(json);
    else logMsg("drop (ws not open): " + json);
  }
  window.__rvpDaemon = {
    send: sendEvent,
    onFrame: function (fn) {
      subs.push(fn);
      return function () { var j = subs.indexOf(fn); if (j >= 0) subs.splice(j, 1); };
    },
    last: function () { return catFrame; }
  };

  // ---- bootstrap: load the SDK, and the dev bridge if its package is installed ----
  function load(src, onerror, onload) {
    var s = document.createElement("script");
    s.src = src;
    s.onerror = onerror;
    if (onload) { s.onload = onload; }
    (document.head || document.getElementsByTagName("head")[0] || document.body).appendChild(s);
  }

  // The daemon relays ctl lines only while cmu-re-dev-bridge is installed, so a line with
  // no bridge loaded means it was installed after this page started (or is still loading):
  // load bridge.js now, hold the lines, hand them over once it is in.
  var BRIDGE = "../../data_persist/cmu-re/pkg/cmu-re-dev-bridge/ui/bridge.js";
  var pend = null;
  function give(s) {
    try { window.__rvpDev.line(s); } catch (e) { logMsg("dev bridge failed: " + e); }
  }
  function loadBridge() {
    load(BRIDGE, function () {
      if (pend) { logMsg("dev bridge not loadable, dropped " + pend.length + " line(s)"); }
      pend = null;
    }, function () {
      var q = pend || [];
      pend = null;
      if (window.__rvpDev) { for (var i = 0; i < q.length; i++) { give(q[i]); } }
    });
  }
  function devLine(s) {
    if (window.__rvpDev) { give(s); return; }
    if (pend) { if (pend.length < 32) { pend.push(s); } return; } // note: cap, a stuck load can't eat RAM
    pend = [s];
    loadBridge();
  }

  load("../../data_persist/cmu-re/pkg/cmu-re-store/ui/sdk.js", function () { logMsg("FAILED to load sdk.js"); });
  // a missing bridge is the normal (release) case: no log
  loadBridge();

  logMsg("injector loaded");
})();
