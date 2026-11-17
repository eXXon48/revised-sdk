/* Revised HMI SDK  (ES5 / Opera-Presto safe, ASCII only)
 * ----------------------------------------------------------------------------
 * Loaded by the injector (developer_mode_GuiConfig.js). Provides the global
 * RevisedHMI: a registry of apps (daemon-built registry.js), menu rows in
 * Applications / Entertainment, and ONE full-screen overlay at a time with an
 * own header + scrollport. App code is lazy-loaded on first row selection.
 * No daemon link, no dev eval bridge here: those stay in the injector.
 */
(function () {
  "use strict";

  var REGISTRY_SRC = "/data_persist/cmu-re/pkg/.state/registry.js";
  var FALLBACK = { pkg: "cmu-re-store", id: "store", section: "applications", label: "Revised Store",
    base: "/data_persist/cmu-re/pkg/cmu-re-store/ui/", main: "controls.js", listen: false };
  var TAG = "[rvp-sdk]";
  // Height of our OWN non-scrolling header. Own bar instead of the stock StatusBarCtrl:
  // its height is context-dependent and its live Home button sat in the inset gap.
  var HEADER_H = 48;

  var entries = [FALLBACK];   // replaced by RevisedHMI_registry(); store row re-ensured after load
  var registered = {};        // key -> {open, close}
  var cur = null;             // the open overlay state, or null
  var pendingKey = null;      // key whose main script is still loading

  var disabled = {};          // key -> true once its package code threw (for the session)
  var listeners = { key: [], screen: [], ready: [] };
  var KNOBS = { 77: 1, 78: 1, 13: 1, 8: 1, 37: 1, 38: 1, 39: 1, 40: 1 };
  var fwDone = false, regDone = false, readyFired = false;

  // log.info when it exists, else console.log; package lines carry the key
  function logMsg(m, key) {
    var line = (key ? "[rvp-sdk " + key + "]" : TAG) + " " + m;
    try {
      if (window.log && window.log.info) window.log.info(line);
      else if (window.console) window.console.log(line);
    } catch (e) {}
  }
  function fail(key, what, e) { disabled[key] = true; logMsg(what + " threw, disabled: " + e, key); }

  function copy(ev) { var o = {}, k; for (k in ev) if (ev.hasOwnProperty(k)) o[k] = ev[k]; return o; }
  // a listener that throws is logged and removed; the others still run.
  // Each listener gets its own copy: one must not change what the next (or the app) sees.
  function fire(name, ev) {
    var list = listeners[name].slice(0), i, j;
    for (i = 0; i < list.length; i++) {
      try { list[i](copy(ev)); }
      catch (e) {
        logMsg(name + " listener threw, removed: " + e);
        j = listeners[name].indexOf(list[i]);
        if (j >= 0) listeners[name].splice(j, 1);
      }
    }
  }
  // listen:true mains load here, before "ready" fires (they hook the framework, which is up now)
  function maybeReady() {
    var i, e;
    if (readyFired || !fwDone || !regDone) return;
    readyFired = true;
    for (i = 0; i < entries.length; i++) {
      e = entries[i];
      if (e.listen) (function (en) {
        loadScript(en.base + en.main, null, function () { logMsg("FAILED to load " + en.base + en.main, en.pkg + "/" + en.id); });
      })(e);
    }
    fire("ready", {});
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

  function consumeRegistry() {
    ensureStore();
    regDone = true;
    maybeReady();
  }

  function loadScript(src, onload, onerror) {
    var s = document.createElement("script");
    s.src = src; s.onload = onload; s.onerror = onerror;
    (document.head || document.getElementsByTagName("head")[0] || document.body).appendChild(s);
  }

  // ---- registry ------------------------------------------------------------
  function ensureStore() {
    var i;
    for (i = 0; i < entries.length; i++) if (entries[i].pkg === FALLBACK.pkg && entries[i].id === FALLBACK.id) return;
    entries.unshift(FALLBACK);
  }
  window.RevisedHMI_registry = function (o) {
    if (o && o.entries && o.entries.length !== undefined) entries = o.entries.slice(0);
    ensureStore();
  };
  function findEntry(key) {
    var i;
    for (i = 0; i < entries.length; i++) if (entries[i].pkg + "/" + entries[i].id === key) return entries[i];
    return null;
  }

  // ---- menu rows -----------------------------------------------------------
  function rowOf(e) {
    var key = e.pkg + "/" + e.id;
    return { appData: { appName: "rvp:" + key, isVisible: true, audioSourceId: "None", mmuiEvent: "RvpOpen:" + key },
      text1Id: null, text1: e.label, disabled: false, itemStyle: "style01", hasCaret: e.section === "applications" };
  }
  // Wrap a section's data-list builder: push our rows AFTER the stock build (the stock
  // whitelist can't drop us), only for the matching ctxtId (the same readyFunction also
  // serves "VehicleStatusMonitor"), and refresh itemCount (the control reads it).
  function wrapBuilder(app, fname, ctxtId, section) {
    var orig = app[fname];
    if (typeof orig !== "function") { logMsg("no " + fname + " to wrap"); return; }
    app[fname] = function () {
      var dl = orig.apply(this, arguments);
      try {
        var c = this._currentContext && this._currentContext.ctxtId, i;
        if (dl && dl.items && dl.items.push && c === ctxtId) {
          for (i = 0; i < entries.length; i++) if (entries[i].section === section) dl.items.push(rowOf(entries[i]));
          dl.itemCount = dl.items.length;
        }
      } catch (e) { logMsg("row append failed: " + e); }
      return dl;
    };
  }
  function patchSystemApp(app) {
    if (app.__rvpPatched) return;
    app.__rvpPatched = true;
    wrapBuilder(app, "_buildApplicationsDataList", "Applications", "applications");
    wrapBuilder(app, "_buildEntertainmentDataList", "Entertainment", "entertainment");
    logMsg("systemApp data-list builders wrapped");
  }

  // Swallow our row's selection (GUI->native goes through sendEventToMmui); all else passes.
  function patchSendEvent(fw) {
    if (fw.__rvpSendPatched) return;
    fw.__rvpSendPatched = true;
    var orig = fw.sendEventToMmui;
    fw.sendEventToMmui = function (uiaId, eventId) {
      if (typeof eventId === "string" && eventId.indexOf("RvpOpen:") === 0) { openKey(eventId.substring(8)); return; }
      return orig.apply(this, arguments);
    };
  }
  // Close on ANY screen change except "Idle" (a background app parking itself).
  function patchContextChange(fw) {
    if (fw.__rvpCtxPatched) return;
    var orig = fw._contextChange;
    if (typeof orig !== "function") { logMsg("no _contextChange to wrap"); return; }
    fw.__rvpCtxPatched = true;
    fw._contextChange = function (o) {
      var idle = o && o.ctxtId === "Idle";
      if (!idle && (cur || pendingKey)) { try { closeOverlay(); } catch (e) {} }
      if (!idle && o) fire("screen", { uiaId: o.uiaId, ctxtId: o.ctxtId });
      return orig.apply(this, arguments);
    };
  }

  // ---- scroll: one pending Y, one frame apply ------------------------------
  function frame(fn) {
    if (window.requestAnimationFrame) window.requestAnimationFrame(fn); else setTimeout(fn, 16);
  }
  function setPending(st, y) {
    st.pendingY = y;
    if (st.queued) return;
    st.queued = true;
    frame(function () {
      st.queued = false;
      if (cur !== st || st.pendingY === null) return;
      var max = st.port.scrollHeight - st.port.clientHeight, v = st.pendingY;
      if (v > max) v = max;
      if (v < 0) v = 0;
      st.pendingY = null;
      st.port.scrollTop = v;
    });
  }

  // ---- overlay -------------------------------------------------------------
  function buildOverlay(entry, key, app) {
    var st = { key: key, app: app, pendingY: null, queued: false, timer: null };
    var overlay = document.createElement("div");
    overlay.id = "rvp-overlay";
    var s = overlay.style;
    s.position = "fixed"; s.left = "0"; s.top = "0"; s.right = "0"; s.bottom = "0";
    // overflow:hidden => no scrollbar on the fixed 800x480 screen
    s.zIndex = "99999"; s.background = "#000"; s.overflow = "hidden";
    document.body.appendChild(overlay);
    st.overlay = overlay;

    // OWN header (non-scrolling sibling of the scrollport): title, headerSlot, clock, back
    var header = document.createElement("div");
    var hs = header.style;
    hs.position = "absolute"; hs.left = "0"; hs.top = "0"; hs.right = "0"; hs.height = HEADER_H + "px";
    hs.background = "#000"; hs.borderBottom = "1px solid #1c1c1c"; hs.zIndex = "2";
    var title = document.createElement("span");
    title.id = "rvp-inj-title";
    title.textContent = entry.label;
    var tst = title.style;
    tst.position = "absolute"; tst.left = "60px"; tst.top = "0"; tst.lineHeight = HEADER_H + "px";   // 60px: room for the app's burger
    tst.color = "#fff"; tst.fontSize = "22px";
    var slot = document.createElement("span");
    slot.id = "rvp-sdk-slot";
    var sst = slot.style;
    sst.position = "absolute"; sst.right = "140px"; sst.top = "0"; sst.height = HEADER_H + "px"; sst.lineHeight = HEADER_H + "px";
    sst.color = "#fff"; sst.fontSize = "18px";
    var clock = document.createElement("span");
    var cst = clock.style;
    cst.position = "absolute"; cst.right = "64px"; cst.top = "0"; cst.lineHeight = HEADER_H + "px";   // 64px: room for the back button
    cst.color = "#fff"; cst.fontSize = "22px";
    var back = document.createElement("span");
    back.id = "rvp-sdk-back";
    // owner-picked rounded-square back glyph; evenodd cuts the arrow out of the square
    back.innerHTML = '<svg viewBox="0 0 24 24" width="28" height="28" style="display:block;margin:10px 14px;fill:#FFFFFF" xmlns="http://www.w3.org/2000/svg"><path fill-rule="evenodd" d="M16.19 2H7.81C4.17 2 2 4.17 2 7.81V16.18C2 19.83 4.17 22 7.81 22H16.18C19.82 22 21.99 19.83 21.99 16.19V7.81C22 4.17 19.83 2 16.19 2ZM13.92 16.13H9C8.59 16.13 8.25 15.79 8.25 15.38C8.25 14.97 8.59 14.63 9 14.63H13.92C15.2 14.63 16.25 13.59 16.25 12.3C16.25 11.01 15.21 9.97 13.92 9.97H8.85L9.11 10.23C9.4 10.53 9.4 11 9.1 11.3C8.95 11.45 8.76 11.52 8.57 11.52C8.38 11.52 8.19 11.45 8.04 11.3L6.47 9.72C6.18 9.43 6.18 8.95 6.47 8.66L8.04 7.09C8.33 6.8 8.81 6.8 9.1 7.09C9.39 7.38 9.39 7.86 9.1 8.15L8.77 8.48H13.92C16.03 8.48 17.75 10.2 17.75 12.31C17.75 14.42 16.03 16.13 13.92 16.13Z"/></svg>';
    var bst = back.style;
    bst.position = "absolute"; bst.right = "0"; bst.top = "0"; bst.width = "56px"; bst.height = HEADER_H + "px";
    bst.lineHeight = HEADER_H + "px"; bst.textAlign = "center"; bst.color = "#fff"; bst.fontSize = "26px";
    back.onclick = function () { if (cur === st) overlayKey(st, { key: 8, phase: "down" }); };
    header.appendChild(title); header.appendChild(slot); header.appendChild(clock); header.appendChild(back);
    function pad2(n) { return (n < 10 ? "0" : "") + n; }
    function tickClock() { try { var d = new Date(); clock.textContent = pad2(d.getHours()) + ":" + pad2(d.getMinutes()); } catch (e) {} }
    tickClock(); st.timer = setInterval(tickClock, 15000);   // note: minutes only, 15s poll is plenty
    overlay.appendChild(header);

    // scroll area below the header; the app draws into host (host.parentNode = viewport)
    var port = document.createElement("div");
    var ps = port.style;
    ps.position = "absolute"; ps.left = "0"; ps.right = "0"; ps.top = HEADER_H + "px"; ps.bottom = "0"; ps.overflow = "hidden";
    var host = document.createElement("div");
    port.appendChild(host);
    overlay.appendChild(port);
    st.port = port; st.host = host; st.title = title; st.slot = slot;

    // finger/mouse drag scroll. The embedded Opera delivers a finger drag as MOUSE
    // events, not touch, so bind BOTH. The SDK is the one scroll owner: drag sets the
    // pending Y and the shared per-frame apply scrolls the port (apps use ctx.scrollTo).
    // The release is bound on the overlay: a finger that slides off the scrollport
    // (onto the header) still ends the drag.
    // A move threshold keeps taps clicking; a capture-phase click swallows the click
    // that ends a real drag. note: 1:1 drag, no flick momentum.
    // DEBUG mm-probe: per gesture, count mousemoves and their gaps (Opera replaying a
    // backed-up queue shows as sub-4ms bursts). Goes out via RevisedHMI._send if the
    // injector set it. Revert: delete the mm* block + its 3 calls, and the ptr send in dragDown.
    // RELEASE GATE: removed together with the DEV-ONLY eval bridge (roadmap S4 release gates).
    var lastY = 0, startY = 0, startTop = 0, dragging = false, dragMoved = 0, dragLogged = false;
    var mmN = 0, mmT0 = 0, mmTprev = 0, mmBurst = 0, mmStall = 0, mmMin = 99999;
    function send(evt) { if (window.RevisedHMI && window.RevisedHMI._send) { try { window.RevisedHMI._send(evt); } catch (e) {} } }
    function mmReset() { mmN = 0; mmT0 = 0; mmTprev = 0; mmBurst = 0; mmStall = 0; mmMin = 99999; }
    function mmTick() {
      var t = +new Date();
      if (!mmN) { mmT0 = t; }
      else { var g = t - mmTprev; if (g < mmMin) mmMin = g; if (g < 4) mmBurst++; if (g > 30) mmStall++; }
      mmTprev = t; mmN++;
    }
    function mmReport() {
      send({ control: "env", action: "mm",
        value: "n=" + mmN + " span=" + (mmTprev - mmT0) + "ms min=" + mmMin + " burst<4ms=" + mmBurst + " stall>30ms=" + mmStall,
        ts: Date.now() });
    }
    function dragPos(e) { return (e.touches && e.touches[0]) ? e.touches[0].clientY : e.clientY; }
    function dragDown(e) {
      if (!dragLogged) { dragLogged = true; send({ control: "env", action: "ptr", value: e.type, ts: Date.now() }); }
      dragging = true; dragMoved = 0; lastY = startY = dragPos(e); mmReset();
      startTop = st.pendingY !== null ? st.pendingY : port.scrollTop;
      // flag the content for the whole drag so a CSS :not(.rvp-dragging) hover gate stops matching
      if (host.className.indexOf("rvp-dragging") < 0) host.className += " rvp-dragging";
    }
    function dragMove(e) {
      mmTick();
      if (!dragging) return;
      var y = dragPos(e); dragMoved += Math.abs(lastY - y); lastY = y;
      setPending(st, startTop + (startY - y));
      if (dragMoved > 5 && e.preventDefault) e.preventDefault();
    }
    function dragEnd() {
      if (!dragging) return;
      dragging = false;
      host.className = host.className.replace(/\s*rvp-dragging\b/g, "");
      mmReport();
    }
    port.addEventListener("touchstart", dragDown, false);
    port.addEventListener("touchmove", dragMove, false);
    overlay.addEventListener("touchend", dragEnd, false);
    port.addEventListener("mousedown", dragDown, false);
    port.addEventListener("mousemove", dragMove, false);
    overlay.addEventListener("mouseup", dragEnd, false);
    port.addEventListener("click", function (e) {
      if (dragMoved > 8) { if (e.preventDefault) e.preventDefault(); if (e.stopPropagation) e.stopPropagation(); dragMoved = 0; }
    }, true);
    return st;
  }

  function openOverlay(entry, key, app) {
    var st = buildOverlay(entry, key, app);
    cur = st;
    var ctx = {
      root: st.host,
      assets: entry.base,
      headerSlot: st.slot,
      setTitle: function (t) { st.title.textContent = t; },
      onKey: function (fn) { st.keyFn = typeof fn === "function" ? fn : null; },
      close: function () { if (cur === st) closeOverlay(); },
      scrollTo: function (y) { if (cur === st) setPending(st, y); }
    };
    try { app.open(ctx); }   // its return value is not used: the SDK owns drag scrolling
    catch (e) { fail(key, "open", e); closeOverlay(true); return; }
    logMsg("overlay opened " + key);
  }

  // key order steps 2 and 4 for an open overlay; returns whether the app consumed it
  function overlayKey(st, ev) {
    var used = false;
    if (st.keyFn) {
      try { used = st.keyFn(ev) === true; }
      catch (e) { fail(st.key, "onKey", e); st.keyFn = null; if (cur === st) closeOverlay(true); }
    }
    if (!used && ev.key === 8 && ev.phase === "down" && cur === st) closeOverlay();
    return used;
  }
  // stock Multicontroller acts on keyup, so swallow an up iff we swallowed its down
  var held = {};
  function swallow(e) {
    if (e.preventDefault) e.preventDefault();
    if (e.stopPropagation) e.stopPropagation();
    if (e.stopImmediatePropagation) e.stopImmediatePropagation();
  }
  function onKeyEvent(e, phase) {
    var k = e.keyCode || e.which, st, used = false;
    fire("key", { key: k, phase: phase });
    st = cur;
    if (st) used = overlayKey(st, { key: k, phase: phase });   // step 4 (back closes) happens inside, after the app had its say
    if (phase === "up") {
      if (held[k]) { delete held[k]; swallow(e); }
    } else if (st && (used || KNOBS[k])) { held[k] = 1; swallow(e); }
  }
  function patchKeys() {
    window.addEventListener("keydown", function (e) { onKeyEvent(e, "down"); }, true);
    window.addEventListener("keyup", function (e) { onKeyEvent(e, "up"); }, true);
    // a keyup that went elsewhere (focus lost) never comes: forget the swallowed downs
    window.addEventListener("blur", function () { held = {}; }, true);
  }

  function closeOverlay(quiet) {
    pendingKey = null;
    var st = cur;
    if (!st) return;
    cur = null;
    held = {};   // a down whose up never comes would eat a later stock up of that code
    if (st.app && st.app.close) { try { st.app.close(); } catch (e) { if (!quiet) fail(st.key, "close", e); } }
    if (st.timer) { try { clearInterval(st.timer); } catch (e2) {} st.timer = null; }
    st.slot.innerHTML = "";
    if (st.overlay.parentNode) st.overlay.parentNode.removeChild(st.overlay);
    if (!quiet) logMsg("overlay closed " + st.key);
  }

  function openKey(key) {
    if (disabled[key]) { logMsg("disabled, not opening", key); return; }
    closeOverlay();   // only one overlay at a time
    var entry = findEntry(key);
    if (!entry) { logMsg("unknown app " + key); return; }
    if (registered[key]) { openOverlay(entry, key, registered[key]); return; }
    pendingKey = key;
    loadScript(entry.base + entry.main, function () {
      if (pendingKey !== key) return;   // closed/superseded while loading
      pendingKey = null;
      if (!registered[key]) { logMsg(key + ": main loaded but did not call RevisedHMI.register"); return; }
      openOverlay(entry, key, registered[key]);
    }, function () {
      if (pendingKey === key) pendingKey = null;
      logMsg(key + ": FAILED to load " + entry.base + entry.main);
    });
  }

  window.RevisedHMI = {
    apiVersion: 1,
    register: function (key, app) { registered[key] = app; },
    open: openKey,
    log: function (m, key) { logMsg(m, key || (cur && cur.key)); },   // untagged with no key and no overlay
    on: function (name, fn) {
      if (!listeners[name] || typeof fn !== "function") return;
      if (name === "ready" && readyFired) { fire1(fn); return; }   // late subscriber (listen:true main loads async)
      listeners[name].push(fn);
    }
  };
  function fire1(fn) { try { fn({}); } catch (e) { logMsg("ready listener threw: " + e); } }

  // ---- bootstrap -----------------------------------------------------------
  loadScript(REGISTRY_SRC, consumeRegistry, function () { logMsg("registry.js not loaded, store row only"); consumeRegistry(); });
  waitFor(function () { return window.framework && window.framework.sendEventToMmui ? window.framework : null; },
    function (fw) { patchSendEvent(fw); patchContextChange(fw); patchKeys(); fwDone = true; maybeReady(); });
  // the system app owns the Applications / Entertainment menus (framework.getAppInstance, see injector)
  waitFor(function () {
    try {
      var app = (window.framework && window.framework.getAppInstance) ? window.framework.getAppInstance("system") : null;
      return (app && typeof app._buildApplicationsDataList === "function") ? app : null;
    } catch (e) { return null; }
  }, patchSystemApp);
})();
