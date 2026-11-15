// headless logic check for deploy/sdk.js (registry, rows, overlay lifecycle). Not deployed.
// run: node ui/webapp/test/check-sdk.js ui/deploy/sdk.js
var assert = require("assert"), path = require("path"), fs = require("fs"), vm = require("vm");
var SRC = path.resolve(process.argv[2] || path.join(__dirname, "..", "..", "deploy", "sdk.js"));
var code = fs.readFileSync(SRC, "utf8");
assert(!fs.readFileSync(SRC).some(function (b) { return b > 127; }), "sdk.js must be ASCII-only");

function Node(tag) { this.tagName = tag; this.children = []; this.parentNode = null; this.className = ""; this.style = {}; this.textContent = ""; this.id = ""; this.scrollTop = 0; this.scrollHeight = 1000; this.clientHeight = 400; this.ev = {}; }
Node.prototype.appendChild = function (c) { c.parentNode = this; this.children.push(c); return c; };
Node.prototype.removeChild = function (c) { this.children.splice(this.children.indexOf(c), 1); c.parentNode = null; };
Node.prototype.addEventListener = function (t, f) { this.ev[t] = f; };
Object.defineProperty(Node.prototype, "innerHTML", { set: function () { this.children = []; }, get: function () { return ""; } });
function find(n, id) { if (n.id === id) return n; for (var i = 0; i < n.children.length; i++) { var r = find(n.children[i], id); if (r) return r; } return null; }
function inside(n, anc) { while (n) { if (n === anc) return true; n = n.parentNode; } return false; }

var logs, frames, body, head, fw, sysApp, calls, wl, intervals;
function mkList(name) { return function () { return { itemCount: 1, items: [{ stock: name }] }; }; }
function load(lateFw) {   // lateFw: window.framework appears only after the first waitFor poll
  logs = []; frames = []; calls = []; wl = {}; intervals = []; body = new Node("body"); head = new Node("head");
  global.window = { log: { info: function (m) { logs.push(m); } }, requestAnimationFrame: function (f) { frames.push(f); },
    addEventListener: function (t, f, cap) { assert.strictEqual(cap, true, "capture"); assert(!wl[t], "one " + t + " listener"); wl[t] = f; } };
  global.document = { body: body, head: head, createElement: function (t) { return new Node(t); }, getElementById: function (id) { return find(body, id); } };
  global.setInterval = function (f) { intervals.push(f); try { f(); } catch (e) {} return 1; };   // waitFor polls once at once
  global.clearInterval = function () {};
  global.setTimeout = function (f) { frames.push(f); return 1; };
  sysApp = { _currentContext: { ctxtId: "Applications" }, _buildApplicationsDataList: mkList("a"), _buildEntertainmentDataList: mkList("e") };
  fw = { sendEventToMmui: function () { calls.push([].slice.call(arguments)); }, getAppInstance: function (id) { return id === "system" ? sysApp : null; },
    _contextChange: function (o) { calls.push(["ctx", o.ctxtId]); } };
  if (!lateFw) window.framework = fw;
  vm.runInThisContext(code, { filename: SRC });
}
function scripts() { return head.children.filter(function (c) { return c.tagName === "script"; }); }
function regScript() { return scripts()[0]; }
var STORE = { pkg: "cmu-re-store", id: "store", section: "applications", label: "Revised Store", base: "/data_persist/cmu-re/pkg/cmu-re-store/ui/", main: "controls.js", listen: false };
var P1 = { pkg: "p1", id: "dash", section: "applications", label: "Dash", base: "/p1/ui/", main: "m.js", listen: false };
var P2 = { pkg: "p2", id: "ent", section: "entertainment", label: "Ent", base: "/p2/ui/", main: "e.js", listen: false };
function reg(entries) { window.RevisedHMI_registry({ apiVersion: 1, entries: entries }); regScript().onload(); }
function rows(fn, ctxt) { sysApp._currentContext.ctxtId = ctxt; return sysApp[fn](); }
function rowOf(e) { return { appData: { appName: "rvp:" + e.pkg + "/" + e.id, isVisible: true, audioSourceId: "None", mmuiEvent: "RvpOpen:" + e.pkg + "/" + e.id },
  text1Id: null, text1: e.label, disabled: false, itemStyle: "style01", hasCaret: e.section === "applications" }; }
function key(code, phase) {   // fire a window capture keydown/keyup; report what was swallowed
  var ev = { keyCode: code, pd: 0, sp: 0, sip: 0, preventDefault: function () { this.pd++; }, stopPropagation: function () { this.sp++; }, stopImmediatePropagation: function () { this.sip++; } };
  wl[phase === "up" ? "keyup" : "keydown"](ev);
  ev.swallowed = ev.pd > 0 && ev.sp > 0 && ev.sip > 0;
  return ev;
}
function openApp(entry, app) {   // select the row, run main (optionally registering), fire onload
  var key = entry.pkg + "/" + entry.id;
  fw.sendEventToMmui("system", "RvpOpen:" + key);
  var s = scripts()[scripts().length - 1];
  assert.strictEqual(s.src, entry.base + entry.main);
  if (app) window.RevisedHMI.register(key, app);
  s.onload();
}

// registry path constant
load(); assert.strictEqual(regScript().src, "/data_persist/cmu-re/ui/registry.js");
assert.strictEqual(window.RevisedHMI.apiVersion, 1);

// rows_by_section
load(); reg([STORE, P2]);   // applications entry is the store itself: a registry without it gets the fallback row added
var a = rows("_buildApplicationsDataList", "Applications");
assert.strictEqual(a.items.length, 2); assert.deepStrictEqual(a.items[1], rowOf(STORE)); assert.strictEqual(a.itemCount, 2);
a = rows("_buildApplicationsDataList", "Entertainment"); assert.strictEqual(a.items.length, 1);
var e = rows("_buildEntertainmentDataList", "Entertainment");
assert.strictEqual(e.items.length, 2); assert.deepStrictEqual(e.items[1], rowOf(P2)); assert.strictEqual(e.itemCount, 2);
assert.strictEqual(rows("_buildEntertainmentDataList", "Applications").items.length, 1);
assert.strictEqual(rows("_buildApplicationsDataList", "VehicleStatusMonitor").items.length, 1);
assert.strictEqual(rows("_buildEntertainmentDataList", "VehicleStatusMonitor").items.length, 1);

// fallback_store_row
function storeRows() { return rows("_buildApplicationsDataList", "Applications").items.filter(function (r) { return r.appData && r.appData.mmuiEvent === "RvpOpen:cmu-re-store/store"; }); }
load(); regScript().onerror(); assert.strictEqual(storeRows().length, 1);
assert.deepStrictEqual(storeRows()[0], rowOf(STORE));
load(); regScript().onload(); assert.strictEqual(storeRows().length, 1);
load(); reg([P1]); assert.strictEqual(storeRows().length, 1);
load(); reg([STORE, P1]); assert.strictEqual(storeRows().length, 1);

// select_opens_lazy
load(); reg([P1]);
var opened = [], appObj = { open: function (c) { opened.push(c); } };
fw.sendEventToMmui("system", "other:evt", 3);
assert.deepStrictEqual(calls, [["system", "other:evt", 3]]);
calls.length = 0;
fw.sendEventToMmui("system", "RvpOpen:p1/dash");
assert.strictEqual(calls.length, 0); assert.strictEqual(scripts().length, 2);
var s = scripts()[1]; assert.strictEqual(s.src, "/p1/ui/m.js");
window.RevisedHMI.register("p1/dash", appObj); s.onload();
assert.strictEqual(opened.length, 1);
var ov = find(body, "rvp-overlay"); assert(ov);
assert(inside(opened[0].root, ov)); assert.strictEqual(opened[0].assets, "/p1/ui/");
assert.strictEqual(find(body, "rvp-inj-title").textContent, "Dash");
opened[0].setTitle("X"); assert.strictEqual(find(body, "rvp-inj-title").textContent, "X");
assert(find(body, "rvp-sdk-back"));

// main_without_register
load(); reg([P1]); var n0 = logs.length;
openApp(P1, null);
assert.strictEqual(find(body, "rvp-overlay"), null); assert.strictEqual(logs.length - n0, 1);
assert.strictEqual(rows("_buildApplicationsDataList", "Applications").items.length, 3);

// context_change_closes
load(); reg([P1]); var closed = 0;
openApp(P1, { open: function () {}, close: function () { closed++; } });
calls.length = 0;
fw._contextChange({ ctxtId: "Idle", uiaId: "system" });
assert(find(body, "rvp-overlay")); assert.strictEqual(closed, 0);
fw._contextChange({ ctxtId: "Home", uiaId: "system" });
assert.strictEqual(find(body, "rvp-overlay"), null); assert.strictEqual(closed, 1);
assert.deepStrictEqual(calls.slice(-1), [["ctx", "Home"]]);

// one_overlay
load(); reg([P1, P2]); var order = [];
window.RevisedHMI.register("p1/dash", { open: function () { order.push("o1"); }, close: function () { order.push("c1"); } });
window.RevisedHMI.register("p2/ent", { open: function () { order.push("o2"); }, close: function () { order.push("c2"); } });
window.RevisedHMI.open("p1/dash"); window.RevisedHMI.open("p2/ent");
assert.deepStrictEqual(order, ["o1", "c1", "o2"]);
assert.strictEqual(body.children.length, 1);

// header_slot
load(); reg([P1]); var ctxs = [];
window.RevisedHMI.register("p1/dash", { open: function (c) { ctxs.push(c); c.headerSlot.appendChild(new Node("b")); } });
window.RevisedHMI.open("p1/dash");
var hdr = find(body, "rvp-inj-title").parentNode, slot = ctxs[0].headerSlot;
assert.strictEqual(slot.parentNode, hdr);
var after = hdr.children[hdr.children.indexOf(slot) + 1];
assert(after && after !== find(body, "rvp-sdk-back") && after !== find(body, "rvp-inj-title"), "clock follows slot");
assert.strictEqual(slot.children.length, 1);
fw._contextChange({ ctxtId: "Home" }); assert.strictEqual(slot.children.length, 0);
window.RevisedHMI.open("p1/dash");
assert.notStrictEqual(ctxs[1].headerSlot, slot); assert.strictEqual(ctxs[1].headerSlot.children.length, 1);

// scroll_to_batched_clamped
load(); reg([P1]); var sc;
window.RevisedHMI.register("p1/dash", { open: function (c) { sc = c; } });
window.RevisedHMI.open("p1/dash");
var port = sc.root.parentNode; port.scrollHeight = 1000; port.clientHeight = 400;
frames.length = 0;
sc.scrollTo(50); sc.scrollTo(120);
assert.strictEqual(port.scrollTop, 0); assert.strictEqual(frames.length, 1);
frames.shift()(); assert.strictEqual(port.scrollTop, 120);
sc.scrollTo(-5); frames.shift()(); assert.strictEqual(port.scrollTop, 0);
sc.scrollTo(99999); frames.shift()(); assert.strictEqual(port.scrollTop, 600);
// drag without hooks shares the one pending apply
sc.scrollTo(100); port.ev.mousedown({ clientY: 300, type: "mousedown" }); port.ev.mousemove({ clientY: 250 });
assert.strictEqual(frames.length, 1); frames.shift()(); assert.strictEqual(port.scrollTop, 150);
find(body, "rvp-overlay").ev.mouseup({});

// one_scroll_owner: whatever open() returns, the SDK drag scrolls the port itself (no app hooks)
load(); reg([P1]); var hk = [], sh;
window.RevisedHMI.register("p1/dash", { open: function (c) { sh = c; return { dragStart: function (y) { hk.push("s" + y); }, dragTo: function (y) { hk.push("t" + y); }, dragStop: function () { hk.push("e"); } }; } });
window.RevisedHMI.open("p1/dash"); port = sh.root.parentNode; frames.length = 0;
port.ev.mousedown({ clientY: 300, type: "mousedown" }); port.ev.mousemove({ clientY: 250 });
assert.strictEqual(sh.root.className.indexOf("rvp-dragging") >= 0, true, "content flagged while dragging");
assert.strictEqual(frames.length, 1); frames.shift()(); assert.strictEqual(port.scrollTop, 50);
assert.deepStrictEqual(hk, [], "app drag hooks are never called");
// drag_end_on_overlay: the release lands outside the scrollport (finger slid onto the header)
assert.strictEqual(port.ev.mouseup, undefined, "release not bound on the scrollport only");
var ovl = find(body, "rvp-overlay"); ovl.ev.mouseup({});
assert.strictEqual(sh.root.className.indexOf("rvp-dragging"), -1, "drag ended");
port.ev.mousemove({ clientY: 100 }); assert.strictEqual(frames.length, 0, "no scroll after the release");
port.ev.touchstart({ touches: [{ clientY: 300 }], type: "touchstart" }); ovl.ev.touchend({});
port.ev.touchmove({ touches: [{ clientY: 100 }] }); assert.strictEqual(frames.length, 0, "touchend on the overlay ends the drag");
sh.scrollTo(70); frames.shift()(); assert.strictEqual(port.scrollTop, 70);

// key_listeners_everywhere
load(); reg([P1]); var kl = [];
window.RevisedHMI.on("key", function (k) { kl.push(k); });
var ke = key(77, "down"); key(77, "up");
assert.deepStrictEqual(kl, [{ key: 77, phase: "down" }, { key: 77, phase: "up" }]);
assert.strictEqual(ke.pd + ke.sp + ke.sip, 0);

// unconsumed_knob_key_swallowed / consumed_back_keeps_overlay
load(); reg([P1]); var kf, seen = [], kl2 = [];
window.RevisedHMI.on("key", function (k) { kl2.push(k.key); });
window.RevisedHMI.register("p1/dash", { open: function (c) { c.onKey(function (k) { seen.push(k); return kf(k); }); } });
window.RevisedHMI.open("p1/dash"); kf = function () { return false; };
"77 78 13 37 38 39 40".split(" ").forEach(function (c) { assert(key(+c, "down").swallowed, "knob down " + c); assert(key(+c, "up").swallowed, "knob up " + c); });
assert(!key(65, "down").swallowed); assert(!key(65, "up").swallowed);
assert(!key(77, "up").swallowed, "up without swallowed down passes");
assert.deepStrictEqual(seen[0], { key: 77, phase: "down" });
seen.length = 0; kl2.length = 0;
assert(key(77, "down").swallowed); assert(find(body, "rvp-overlay")); key(77, "up");
assert.deepStrictEqual(kl2, [77, 77]); assert.deepStrictEqual(seen, [{ key: 77, phase: "down" }, { key: 77, phase: "up" }]);
assert(key(8, "down").swallowed); assert.strictEqual(find(body, "rvp-overlay"), null);
assert(key(8, "up").swallowed, "back up swallowed after close"); assert(!key(8, "up").swallowed, "held cleared");
assert(!key(77, "down").swallowed, "no overlay: nothing swallowed");
// keyup_leak_back_and_enter
assert(!key(13, "up").swallowed, "enter up, no overlay, passes");
window.RevisedHMI.open("p1/dash"); kf = function (k) { return k.key === 8; };
assert(key(8, "down").swallowed); assert(find(body, "rvp-overlay"));
fw._contextChange({ ctxtId: "Home" });

// throwing_onKey_disabled
load(); reg([P1]); var tk = 0;
window.RevisedHMI.register("p1/dash", { open: function (c) { c.onKey(function () { tk++; throw new Error("k"); }); } });
window.RevisedHMI.open("p1/dash");
assert(key(13, "down").swallowed, "knob swallowed despite throw"); assert.strictEqual(find(body, "rvp-overlay"), null);
assert(key(13, "up").swallowed); assert.strictEqual(tk, 1);
window.RevisedHMI.open("p1/dash"); assert.strictEqual(find(body, "rvp-overlay"), null); assert.strictEqual(tk, 1);

// header_back_routes_through_onKey
load(); reg([P1]); var hb, hseen = [];
window.RevisedHMI.register("p1/dash", { open: function (c) { c.onKey(function (k) { hseen.push(k); return hb; }); } });
window.RevisedHMI.open("p1/dash"); hb = true; find(body, "rvp-sdk-back").onclick();
assert.deepStrictEqual(hseen, [{ key: 8, phase: "down" }]); assert(find(body, "rvp-overlay"));
hb = false; find(body, "rvp-sdk-back").onclick();
assert.strictEqual(hseen.length, 2); assert.strictEqual(find(body, "rvp-overlay"), null);

// screen_listener
load(); reg([P1]); var scr = [];
window.RevisedHMI.on("screen", function (s) { scr.push(s); });
fw._contextChange({ ctxtId: "Idle", uiaId: "system"}); assert.strictEqual(scr.length, 0);
fw._contextChange({ ctxtId: "Home", uiaId: "system" });
assert.deepStrictEqual(scr, [{ uiaId: "system", ctxtId: "Home" }]);

// throwing_open_cleans_up
load(); reg([P1]); var ck = 0;
window.RevisedHMI.register("p1/dash", { open: function () { ck++; throw new Error("boom"); } });
var n1 = logs.length; window.RevisedHMI.open("p1/dash");
assert.strictEqual(find(body, "rvp-overlay"), null); assert.strictEqual(logs.length - n1, 1);
assert(/^\[rvp-sdk p1\/dash\]/.test(logs[logs.length - 1]), logs[logs.length - 1]);
assert(!key(77, "down").swallowed);
n1 = logs.length; window.RevisedHMI.open("p1/dash");
assert.strictEqual(ck, 1); assert.strictEqual(find(body, "rvp-overlay"), null);
assert(/disabled/.test(logs.slice(n1).join("|")));

// throwing_listener_disabled
load(); reg([P1]); var bad = 0, good = 0;
window.RevisedHMI.on("key", function () { bad++; throw new Error("x"); });
window.RevisedHMI.on("key", function () { good++; });
key(77, "down"); key(77, "down");
assert.strictEqual(bad, 1); assert.strictEqual(good, 2);

// listen_loaded_at_ready: registry first, framework late -> the listen main loads at "ready"
load(true); var L = { pkg: "p3", id: "bg", section: "applications", label: "Bg", base: "/p3/ui/", main: "b.js", listen: true }, rd = 0, seenScripts = -1;
window.RevisedHMI.on("ready", function () { rd++; seenScripts = scripts().length; });
window.RevisedHMI_registry({ apiVersion: 1, entries: [L, P1] });
regScript().onload();
assert.strictEqual(scripts().length, 1, "registry loaded, framework not ready: no listen main yet"); assert.strictEqual(rd, 0);
window.framework = fw; intervals.forEach(function (f) { f(); });
assert.strictEqual(scripts().length, 2); assert.strictEqual(scripts()[1].src, "/p3/ui/b.js");
assert.strictEqual(rd, 1); assert.strictEqual(seenScripts, 2, "main loaded before ready fires");
regScript().onload(); assert.strictEqual(rd, 1); assert.strictEqual(scripts().length, 2, "loaded once");

// key_event_copies: a listener cannot change what the next listener or the app sees
load(); reg([P1]); var ks = [], ka = [];
window.RevisedHMI.on("key", function (k) { k.key = 99; k.phase = "x"; });
window.RevisedHMI.on("key", function (k) { ks.push(k.key + k.phase); k.key = 98; });
window.RevisedHMI.register("p1/dash", { open: function (c) { c.onKey(function (k) { ka.push(k.key + k.phase); return true; }); } });
window.RevisedHMI.open("p1/dash"); key(77, "down");
assert.deepStrictEqual(ks, ["77down"]); assert.deepStrictEqual(ka, ["77down"]);

// held_cleared_on_close_and_blur: a swallowed down whose up never came must not eat a later stock up
load(); reg([P1]);
window.RevisedHMI.register("p1/dash", { open: function (c) { c.onKey(function () { return true; }); } });
window.RevisedHMI.open("p1/dash");
assert(key(13, "down").swallowed); fw._contextChange({ ctxtId: "Home" });
assert(!key(13, "up").swallowed, "overlay closed by the HMI: held cleared");
window.RevisedHMI.open("p1/dash");
assert(key(77, "down").swallowed); wl.blur();
assert(!key(77, "up").swallowed, "window blur: held cleared");
fw._contextChange({ ctxtId: "Home" });

// log_tags_caller
load(); reg([P1]); var nl = logs.length;
window.RevisedHMI.log("a"); window.RevisedHMI.log("b", "p9/x");
window.RevisedHMI.register("p1/dash", { open: function () {} }); window.RevisedHMI.open("p1/dash");
window.RevisedHMI.log("c");
var lg = logs.slice(nl).filter(function (l) { return / [abc]$/.test(l); });
assert.deepStrictEqual(lg, ["[rvp-sdk] a", "[rvp-sdk p9/x] b", "[rvp-sdk p1/dash] c"]);

console.log("check-sdk ok");
