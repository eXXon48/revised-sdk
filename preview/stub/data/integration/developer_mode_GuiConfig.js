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
 *      PREVIEW STUB: here sdk.js is served from ../../preview/sdk.js (no package jail).
 *      The SDK owns the menu rows, the overlay and the drag scroll; the store app
 *      (ui/controls.js) registers itself on it as "cmu-re-store/store".
 *   2. keeps the WebSocket to the Revised daemon (ws://127.0.0.1:2777) and exposes
 *      it to the store shim as window.__rvpDaemon = {send, onFrame, last}.
 *   3. DEV ONLY: eval bridge (js:<code>), __rvpTap, remote knob keys.
 */
(function () {
  "use strict";

  var DAEMON_URL = "ws://127.0.0.1:2777";
  var STORE_KEY = "cmu-re-store/store";
  var TAG = "[revised-store]";
  function logMsg(m) {
    try { if (window.log && log.info) log.info(TAG + " " + m); } catch (e) {}
    try { if (window.console) console.log(TAG, m); } catch (e2) {}
  }

  // ---- poll helper (no Promise in Presto) ---------------------------------
  function waitFor(test, cb, tries, ms) {
    tries = tries || 200; ms = ms || 100;
    var n = 0;
    var h = setInterval(function () {
      var v;
      try { v = test(); } catch (e) { v = null; }
      if (v) { clearInterval(h); cb(v); }
      else if (++n >= tries) { clearInterval(h); logMsg("waitFor gave up after " + n + " tries"); }
    }, ms);
  }

  // ---- WebSocket to the daemon (lazy, auto-reconnect) ---------------------
  var ws = null, wsErrLogged = false;
  var catFrame = null;   // last daemon `catalog` frame, replayed into a freshly opened overlay
  var subs = [];         // daemon-frame subscribers (the store shim)
  // keep the link up from boot so the daemon can drive us (remote "open"/keys)
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
      // remote knob: daemon relays a bare keyCode line ("77") from its ctl FIFO;
      // replay it as a keydown so controls.js handles it like the real knob.
      ws.onmessage = function (m) {
        if (m.data === "open") { if (window.RevisedHMI) window.RevisedHMI.open(STORE_KEY); return; }   // "open" = show the store without the Apps menu
        // DEV ONLY: "js:<code>" evals in the page, result goes back to the daemon log.
        // Anyone who can reach 127.0.0.1:2777 drives the HMI; remove before release.
        if (typeof m.data === "string" && m.data.substring(0, 3) === "js:") { devEval(m.data.substring(3)); return; }
        // store frames (JSON): cache the catalog, hand every frame to the open renderer
        if (typeof m.data === "string" && m.data.charAt(0) === "{") {
          var f = null;
          try { f = JSON.parse(m.data); } catch (e) { logMsg("bad frame: " + e); return; }
          if (f && f.t === "catalog") catFrame = f;
          toRenderer(f);
          return;
        }
        var k = parseInt(m.data, 10);
        if (k > 0) synthKey(k);
      };
      ws.onclose = function () {
        ws = null;
        // link lost: drop the cached catalog -> store shows "Store service offline"
        if (catFrame) { catFrame = null; toRenderer({ t: "offline" }); }
      };
      ws.onerror = function () { if (!wsErrLogged) logMsg("ws error " + DAEMON_URL); wsErrLogged = true; };
    } catch (e) { logMsg("ws ctor failed: " + e); }
  }
  // DEV ONLY eval bridge: global (indirect) eval; reply {control:"eval",action:"ok|err",value}
  function devEval(code) {
    var ok = true, r;
    try { r = (0, eval)(code); } catch (e) { ok = false; r = String(e); }
    try { r = typeof r === "string" ? r : JSON.stringify(r); } catch (e2) { r = String(r); }
    if (r && r.length > 4000) r = r.substring(0, 4000) + "...";
    try { ws.send(JSON.stringify({ control: "eval", action: ok ? "ok" : "err", value: r === undefined ? "undefined" : r, ts: Date.now() })); } catch (e3) {}
  }
  // DEV ONLY tap at screen x,y: mousedown/mouseup/click on the topmost element there.
  window.__rvpTap = function (x, y) {
    var t = document.elementFromPoint(x, y), i, ty = ["mousedown", "mouseup", "click"], e;
    if (!t) return "nothing at " + x + "," + y;
    for (i = 0; i < 3; i++) {
      e = document.createEvent("MouseEvents");
      e.initMouseEvent(ty[i], true, true, window, 1, x, y, x, y, false, false, false, false, 0, null);
      t.dispatchEvent(e);
    }
    return t.tagName + "." + t.className;
  };
  // Generic Event + keyCode expando: Presto-safe (no KeyboardEvent ctor / initKeyEvent needed).
  function synthKey(k) {
    try {
      var e = document.createEvent("Events");
      e.initEvent("keydown", true, true);
      e.keyCode = k; e.which = k;
      (document.activeElement || document.body).dispatchEvent(e);
      logMsg("remote key " + k);
    } catch (err) { logMsg("remote key failed: " + err); }
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

  // ---- bootstrap: load the SDK, then hand it the daemon send for its pointer probe ----
  (function () {
    var s = document.createElement("script");
    s.src = "../../preview/sdk.js";
    s.onerror = function () { logMsg("FAILED to load sdk.js"); };
    (document.head || document.getElementsByTagName("head")[0] || document.body).appendChild(s);
  })();
  waitFor(function () { return window.RevisedHMI || null; }, function (h) { h._send = sendEvent; });

  logMsg("injector loaded");
})();
