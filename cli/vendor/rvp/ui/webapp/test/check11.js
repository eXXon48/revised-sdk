// headless logic check for controls.js store wiring (minimal stub DOM). Not deployed.
// run: node ui/webapp/test/check11.js   [path/to/controls.js]
var assert = require("assert");
function Node(tag) { this.tagName = tag; this.children = []; this.parentNode = null; this.className = ""; this.style = {}; this._t = ""; this.offsetWidth = 10; this.offsetHeight = 10; this.clientHeight = 400; this.ev = {}; }
Node.prototype.appendChild = function (c) { c.parentNode = this; this.children.push(c); return c; };
Node.prototype.removeChild = function (c) { this.children.splice(this.children.indexOf(c), 1); c.parentNode = null; };
Node.prototype.addEventListener = function (t, f) { this.ev[t] = f; };
Node.prototype.removeEventListener = function (t) { delete this.ev[t]; };
Object.defineProperty(Node.prototype, "innerHTML", { set: function (v) { this.children.forEach(function (c) { c.parentNode = null; }); this.children = []; this._t = ""; this._html = String(v); }, get: function () { return ""; } });
Object.defineProperty(Node.prototype, "textContent", { set: function (s) { this.children.forEach(function (c) { c.parentNode = null; }); this.children = []; this._t = String(s); }, get: function () { return text(this); } });
Node.prototype.getBoundingClientRect = function () { return this._rect || { top: 0, bottom: 10 }; };
function text(n) { if (n.tagName === "#text") return n._t; var s = n._t || ""; for (var i = 0; i < n.children.length; i++) s += text(n.children[i]); return s; }
function all(n, out) { out = out || []; for (var i = 0; i < n.children.length; i++) { out.push(n.children[i]); all(n.children[i], out); } return out; }
function hasCls(n, c) { return (" " + n.className + " ").indexOf(" " + c + " ") >= 0; }
Node.prototype.querySelectorAll = function (sel) {
  var cls = sel.split(",").map(function (s) { return s.trim().slice(1); });
  return all(this).filter(function (n) { return cls.some(function (c) { return hasCls(n, c); }); });
};
var listeners = {}, docEv = {};
global.window = { addEventListener: function (t, f) { listeners[t] = f; }, removeEventListener: function () {}, requestAnimationFrame: function () {} };
global.document = {
  createElement: function (t) { return new Node(t); },
  createTextNode: function (s) { var n = new Node("#text"); n._t = s; return n; },
  addEventListener: function (t, f) { docEv[t] = f; }, removeEventListener: function (t) { delete docEv[t]; },
  getElementById: function () { return null; }, head: new Node("head")
};
// stub canvas: getContext("2d") records every call (no arcTo: calling it throws)
function Ctx2d() { this.calls = []; this.fillStyle = null; this.strokeStyle = null; this.globalAlpha = 1; }
"fillRect beginPath moveTo lineTo arc closePath fill stroke fillText save restore translate".split(" ").forEach(function (f) {
  Ctx2d.prototype[f] = function () { this.calls.push({ f: f, a: [].slice.call(arguments), fs: this.fillStyle, ga: this.globalAlpha }); };
});
Ctx2d.prototype.createLinearGradient = function () {
  var g = { a: [].slice.call(arguments), stops: [], addColorStop: function (o, c) { this.stops.push([o, c]); } };
  this.calls.push({ f: "createLinearGradient", a: g.a, g: g }); return g;
};
var mkEl = document.createElement;
document.createElement = function (t) { var n = mkEl(t); if (t === "canvas") { n.ctx2d = new Ctx2d(); n.getContext = function () { return this.ctx2d; }; } return n; };
// fake clock: setTimeout / clearTimeout / Date.now
var clock = { now: 0, q: [], id: 0 };
global.setTimeout = function (fn, ms) { clock.q.push({ id: ++clock.id, at: clock.now + (ms || 0), fn: fn }); return clock.id; };
global.clearTimeout = function (id) { clock.q = clock.q.filter(function (x) { return x.id !== id; }); };
Date.now = function () { return clock.now; };
function runClock(ms) {
  var end = clock.now + ms, x;
  for (;;) {
    clock.q.sort(function (a, b) { return a.at - b.at || a.id - b.id; });
    x = clock.q[0];
    if (!x || x.at > end) break;
    clock.q.shift(); clock.now = x.at; x.fn();
  }
  clock.now = end;
}
// SVG stars: constant markup set via innerHTML (recorded in _html); count filled / empty
function stars(n, cls) { return [n].concat(all(n)).reduce(function (a, x) { return a + ((x._html || "").split("rvp-star " + cls).length - 1); }, 0); }
var path = require("path"), fs = require("fs");
var SRC = path.resolve(process.argv[2] || path.join(__dirname, "..", "controls.js"));
assert(!fs.readFileSync(SRC).some(function (b) { return b > 127; }), "controls.js must be ASCII-only");
var App = require(SRC);

// ---- fixtures ----------------------------------------------------------------
function I(name, o) {
  var it = { name: name, title: "T " + name, subtitle: "sub " + name, category: "Apps", version: "1.0.0", size: 1, state: "available",
    installed: null, author: "me", description: "d", permissions: [], changes: [], rating: null, icon: false, shots: 0,
    everInstalled: false, depends: "", conflicts: "" }, k;
  for (k in o || {}) it[k] = o[k];
  return it;
}
function C(items, o) {
  var c = { t: "catalog", seq: 1, online: true, items: items, featured: [], news: [], account: { linked: true, name: "Dev user" },
    devmode: false, daemon: "0.3.0", restartPending: [] }, k;
  for (k in o || {}) c[k] = o[k];
  return c;
}
// fresh render with a stub SDK ctx
function mk(cat) {
  var t = { sent: [], titles: [], scrolls: [], closed: 0, keyFn: null };
  t.frame = new Node("div"); t.port = t.frame.appendChild(new Node("div")); t.host = t.port.appendChild(new Node("div"));
  t.slot = new Node("span");
  t.ctx = { root: t.host, assets: "x/", headerSlot: t.slot, setTitle: function (s) { t.titles.push(s); },
    onKey: function (f) { t.keyFn = f; }, scrollTo: function (y) { t.scrolls.push(y); }, close: function () { t.closed++; } };
  t.ctl = App.render(t.host, function (e) { t.sent.push(e); }, { ctx: t.ctx });
  t.key = function (k) { return t.keyFn({ key: k, phase: "down" }); };
  if (cat) t.ctl.onDaemon(cat);
  return t;
}
function store(t) { return t.sent.filter(function (e) { return e.control === "store"; }); }
function lastStore(t) { var s = store(t); return s[s.length - 1]; }
function foc(t) { var f = t.frame.querySelectorAll(".rvp-focus").concat(t.slot.querySelectorAll(".rvp-focus")); return f[0] || null; }
function focText(t) { var f = foc(t); return f ? text(f) : null; }
function section(t, name) {
  t.frame.querySelectorAll(".rvp-burger")[0].onclick();
  var d = t.frame.querySelectorAll(".rvp-ditem").filter(function (n) { return text(n) === name; });
  assert(d.length === 1, "drawer item " + name); d[0].onclick();
}
function card(t, title) { var c = t.host.querySelectorAll(".rvp-card").filter(function (n) { return text(n).indexOf(title) >= 0; }); return c[0]; }
function openPage(t, title) { var c = card(t, title); assert(c, "card " + title); c.onclick(); }
function btns(t) { return t.host.querySelectorAll(".rvp-btn").map(function (b) { return text(b); }); }
function btn(t, label, root) { return (root || t.host).querySelectorAll(".rvp-btn").filter(function (b) { return text(b) === label; })[0]; }
function modalText(t) { var m = t.frame.querySelectorAll(".rvp-modal")[0]; return m && hasCls(m, "rvp-on") ? text(m) : ""; }
function focusCls(t, cls) { for (var i = 0; i < 30; i++) { if (foc(t) && hasCls(foc(t), cls)) return foc(t); t.key(77); } assert.fail("no focusable " + cls); }

// ---- title_per_section ---------------------------------------------------------
var t = mk(C([I("m1", { category: "System mods" })]));
section(t, "SysMods");
assert.strictEqual(t.titles[t.titles.length - 1], "Revised Store - SysMods");
openPage(t, "T m1");
assert.strictEqual(t.titles[t.titles.length - 1], "Revised Store - SysMods", "package page keeps the section title");
assert(text(t.host).indexOf("About") >= 0, "page shown");

// ---- header_slot_online_dev --------------------------------------------------------
t = mk(C([], { online: true, devmode: true }));
var on = t.slot.querySelectorAll(".rvp-online");
assert(on.length === 1 && hasCls(on[0], "on"), "online icon lit");
assert(text(t.slot).indexOf("DEV") >= 0, "DEV badge");
t.ctl.onDaemon(C([], { online: false }));
on = t.slot.querySelectorAll(".rvp-online");
assert(on.length === 1 && on[0].className === "rvp-online", "online icon off");
assert(text(t.slot).indexOf("DEV") < 0, "no DEV when devmode off");

// ---- card_fields -----------------------------------------------------------------
t = mk(C([I("a1", { rating: { avg: 4.6, count: 23 }, state: "update", installed: "0.9.0", icon: true }), I("a2")]));
var cd = card(t, "T a1"), ct = text(cd);
assert(ct.indexOf("T a1") >= 0 && ct.indexOf("sub a1") >= 0 && ct.indexOf("4.6") >= 0, ct);
assert.strictEqual(cd.querySelectorAll(".rvp-bi-upd").length, 1, "update: icon");
assert.strictEqual(cd.querySelectorAll(".rvp-btn").length, 0, "no button on the card");
var icoReq = store(t).filter(function (e) { return e.action === "asset"; });
assert.strictEqual(icoReq.length, 1);
assert.deepStrictEqual(icoReq[0].value, { name: "a1", kind: "icon", i: 0 });
t.ctl.onDaemon(C([I("a1", { rating: { avg: 4.6, count: 23 }, state: "update", installed: "0.9.0", icon: true }), I("a2")]));
assert.strictEqual(store(t).filter(function (e) { return e.action === "asset"; }).length, 1, "icon asked once");
t.ctl.onDaemon({ t: "asset", name: "a1", kind: "icon", i: 0, src: "/c/a1/icon-0.png" });
var imgs = all(card(t, "T a1")).filter(function (n) { return n.tagName === "img"; });
assert(imgs.length === 1 && imgs[0].src === "/c/a1/icon-0.png", "card shows the icon src");
assert(text(card(t, "T a2")).indexOf("Installed") < 0 && text(card(t, "T a2")).indexOf("Update") < 0, "available: no badge");

// ---- back_pops_then_closes ------------------------------------------------------------
t = mk(C([I("a1")]));
openPage(t, "T a1");
assert(!card(t, "T a1"), "list replaced by the page");
assert.strictEqual(t.key(8), true, "back on a page is consumed");
assert(card(t, "T a1"), "list shown again");
assert(hasCls(foc(t), "rvp-card"), "focus back on the card");
assert.strictEqual(t.key(8), false, "back at the section root goes to the SDK");
assert.strictEqual(t.closed, 0);

// ---- page_buttons_by_state ------------------------------------------------------------------
t = mk(C([I("a1"), I("a2", { state: "installed", installed: "1.0.0" }), I("a3", { state: "update", installed: "0.9.0" })]));
openPage(t, "T a1"); assert.deepStrictEqual(btns(t), ["Install"]); t.key(8);
openPage(t, "T a2"); assert.deepStrictEqual(btns(t), ["Remove"]); t.key(8);
openPage(t, "T a3"); assert.deepStrictEqual(btns(t), ["Update", "Remove"]);
btn(t, "Remove").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["uninstall", { name: "a3" }]);

// ---- page_buttons_unlinked_offline ----------------------------------------------------------
var three = [I("a1"), I("a2", { state: "installed", installed: "1.0.0" }), I("a3", { state: "update", installed: "0.9.0" })];
t = mk(C(three, { account: { linked: false, name: null } }));
openPage(t, "T a1"); assert.deepStrictEqual(btns(t), ["Sign in to install"]); t.key(8);
openPage(t, "T a2"); assert.deepStrictEqual(btns(t), ["Remove"]); t.key(8);
openPage(t, "T a3"); assert.deepStrictEqual(btns(t), ["Sign in to update", "Remove"]); t.key(8);
openPage(t, "T a1"); var n0 = t.sent.length;
btn(t, "Sign in to install").onclick();
assert(t.host.querySelectorAll(".rvp-pair").length === 1, "Sign in pushes the Pair screen");
assert(store({ sent: t.sent.slice(n0) }).every(function (e) { return e.action !== "plan" && e.action !== "install"; }));
assert.strictEqual(t.key(8), true); assert(text(t.host).indexOf("About") >= 0, "Pair pops back to the page");
t = mk(C(three, { online: false }));
openPage(t, "T a1"); assert.deepStrictEqual(btns(t), ["Offline"]);
assert(hasCls(btn(t, "Offline"), "dis") && !hasCls(btn(t, "Offline"), "rvp-act"), "Offline is disabled");
t.key(8);
openPage(t, "T a3"); assert.deepStrictEqual(btns(t), ["Offline", "Remove"]); t.key(8);
openPage(t, "T a2"); assert.deepStrictEqual(btns(t), ["Remove"]);

// ---- page_sections -----------------------------------------------------------------------------
t = mk(C([I("a1", { permissions: [], depends: "", changes: ["2.0.0: faster", "1.0.0: first"], files: ["/jci/a", "/jci/b"], size: 2048 }),
  I("a2", { permissions: ["audio", "input"], depends: "t-basic (>= 1.0)", conflicts: "t-x" }), I("a3")]));
openPage(t, "T a1");
var pt = text(t.host);
assert(pt.indexOf("Permissions: None") >= 0, pt);
assert(pt.indexOf("Requires") < 0 && pt.indexOf("Conflicts") < 0, "empty depends/conflicts hidden");
assert(pt.indexOf("2 KB") >= 0 && pt.indexOf("v1.0.0") >= 0, "stats: size, version");
var accs = t.host.querySelectorAll(".rvp-acc").map(text);
assert.deepStrictEqual(accs, ["About", "What's new", "What it changes"]);
var bodies = t.host.querySelectorAll(".rvp-accb");
assert.deepStrictEqual(bodies.map(function (b) { return b.style.display; }), ["block", "none", "none"], "About open, rest collapsed");
focusCls(t, "rvp-acc"); t.key(77); t.key(13);   // About -> What's new
assert.strictEqual(t.host.querySelectorAll(".rvp-accb")[1].style.display, "block", "13 expands");
assert(text(bodies[1]).indexOf("2.0.0: faster") >= 0);
assert(text(bodies[2]).indexOf("Overrides 2 stock files:") >= 0 && text(bodies[2]).indexOf("/jci/b") >= 0);
t.key(8); openPage(t, "T a2"); pt = text(t.host);
assert(pt.indexOf("Requires: t-basic (>= 1.0)") >= 0 && pt.indexOf("Conflicts: t-x") >= 0, pt);
assert(/Permissions: \w+, \w+/.test(pt), pt);
t.key(8); openPage(t, "T a3");
assert.deepStrictEqual(t.host.querySelectorAll(".rvp-acc").map(text), ["About"], "nothing new or changed: only About");

// ---- rate_visibility ---------------------------------------------------------------------------
t = mk(C([I("a1", { everInstalled: true }), I("a2")]));
openPage(t, "T a1"); assert(btns(t).indexOf("Rate") >= 0, "linked + everInstalled: Rate"); t.key(8);
openPage(t, "T a2"); assert(btns(t).indexOf("Rate") < 0, "never installed: no Rate"); t.key(8);
t.ctl.onDaemon(C([I("a1", { everInstalled: true })], { account: { linked: false } }));
openPage(t, "T a1"); assert(btns(t).indexOf("Rate") < 0, "unlinked: no Rate"); t.key(8);
t.ctl.onDaemon(C([I("a1", { everInstalled: true })]));
openPage(t, "T a1");
assert(t.host.querySelectorAll(".rvp-phead")[0].querySelectorAll(".rvp-rbtn").length === 1, "Rate sits in the header");
assert(text(t.host.querySelectorAll(".rvp-phead")[0]).indexOf("No reviews") >= 0, "no rating yet: No reviews");
btn(t, "Rate").onclick();
function pstars(x) { return x.frame.querySelectorAll(".rvp-modal")[0].querySelectorAll(".rvp-pstar"); }
function filled(x) { return pstars(x).map(function (n) { return stars(n, "on"); }).join(""); }
assert.strictEqual(pstars(t).length, 5, "star picker open: five stars");
assert.strictEqual(text(t.frame.querySelectorAll(".rvp-modal")[0]), "", "stars only, no text");
assert.strictEqual(filled(t), "10000", "not rated: focus on star 1");
t.key(77); t.key(77); t.key(77);
assert.strictEqual(filled(t), "11110", "knob fills up to the focused star");
t.key(13);
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["rate", { name: "a1", stars: 4 }]);
assert.strictEqual(pstars(t).length, 0, "picker closed after rating");
btn(t, "Rate").onclick(); n0 = t.sent.length;
assert.strictEqual(t.key(8), true); assert.strictEqual(pstars(t).length, 0, "back cancels the picker");
assert(store({ sent: t.sent.slice(n0) }).length === 0, "cancel sends nothing");

// ---- screenshots_row: every shot is a thumbnail on the page; Enter opens the viewer on that shot ----
t = mk(C([I("a1", { shots: 3 })]));
openPage(t, "T a1");
var shots = store(t).filter(function (e) { return e.action === "asset"; });
assert.deepStrictEqual(shots.map(function (e) { return e.value.i; }), [0, 1, 2], "all three thumbs asked on open");
assert.strictEqual(t.host.querySelectorAll(".rvp-shot").length, 3, "three thumbs");
t.ctl.onDaemon({ t: "asset", name: "a1", kind: "shot", i: 1, error: "fetch failed" });
assert.strictEqual(t.host.querySelectorAll(".rvp-grey").length, 1, "missing image = grey box");
focusCls(t, "rvp-shot"); t.key(77);   // second thumb
// ---- screenshot_viewer: knob flips there, Back closes, cached shots not asked again ----
t.key(13);
assert(hasCls(mroot(t), "rvp-mshot") && modalText(t).indexOf("2/3") >= 0, "viewer open on shot 2");
t.key(77); assert(modalText(t).indexOf("3/3") >= 0, "knob CW flips in viewer");
t.key(37); t.key(37); assert(modalText(t).indexOf("1/3") >= 0, "37 flips back");
assert.strictEqual(store(t).filter(function (e) { return e.action === "asset"; }).length, 3, "no shot asked twice");
t.key(8);
assert.strictEqual(modalText(t), "", "Back closes viewer");
t.key(13); mroot(t).querySelectorAll(".rvp-mx")[0].onclick();
assert.strictEqual(modalText(t), "", "close button closes viewer");

// ---- config_button: installed package with a config gets Config next to Remove ----
var CFG = { fields: [
  { key: "accent", type: "select", label: "Accent colour", default: "#FF2A30",
    options: [{ label: "Red", value: "#FF2A30" }, { label: "Green", value: "#2ECC71" }, { label: "Blue", value: "#2E86FF" }] },
  { key: "size", type: "range", label: "Font size", default: 16, min: 12, max: 24, step: 2 },
  { key: "count", type: "number", label: "Rows", default: 3, min: 1, max: 9 },
  { key: "clock", type: "switch", label: "Show clock", default: true, on: "1", off: "0" }],
  values: { accent: "#FF2A30", size: 16, count: 3, clock: true } };
function cfgItem() { var c = { fields: CFG.fields, values: {} }, k; for (k in CFG.values) c.values[k] = CFG.values[k]; return I("skin", { state: "installed", installed: "1.0.0", config: c, title: "Skin" }); }
function cfgRows(x) { return x.host.querySelectorAll(".rvp-cfg").map(function (r) { var v = r.querySelectorAll(".rvp-cfgval")[0]; return hasCls(v, "rvp-sw") ? (hasCls(v, "on") ? "On" : "Off") : hasCls(v, "rvp-num") ? text(v.querySelectorAll(".rvp-numv")[0]) : text(v); }); }
t = mk(C([cfgItem()]));
openPage(t, "Skin");
assert(btns(t).indexOf("Configuration") >= 0 && btns(t).indexOf("Remove") >= 0, btns(t).join());
t = mk(C([I("skin", { state: "installed", installed: "1.0.0", title: "Skin" }), I("av", { config: CFG, title: "Av" })]));
openPage(t, "Skin"); assert(btns(t).indexOf("Configuration") < 0, "no config = no button");
t.key(8); openPage(t, "Av"); assert(btns(t).indexOf("Configuration") < 0, "not installed = no button");
// ---- config_screen_rows ----
t = mk(C([cfgItem()]));
openPage(t, "Skin"); btn(t, "Configuration").onclick();
assert.deepStrictEqual(cfgRows(t), ["Red", "16", "3", "On"]);
var sw = t.host.querySelectorAll(".rvp-cfg")[3].querySelectorAll(".rvp-cfgval")[0];
assert(hasCls(sw, "rvp-sw") && sw.querySelectorAll(".rvp-swk").length === 1 && text(sw) === "", "switch = toggle (track + knob), no On/Off text");
function rngFill(x) { return x.host.querySelectorAll(".rvp-cfg")[1].querySelectorAll(".rvp-rngf")[0].style.width; }
assert.strictEqual(rngFill(t), "33%", "range = track filled to the value (16 of 12..24)");
assert.strictEqual(t.host.querySelectorAll(".rvp-cfg")[1].querySelectorAll(".rvp-rngk")[0].style.left, "33%", "thumb at the value");
assert.strictEqual(t.host.querySelectorAll(".rvp-cfg")[2].querySelectorAll(".rvp-rng").length, 0, "number has no slider");
// ---- config_number_buttons: [-] value [+], a tap steps by 1 within min..max ----
function numBtn(x, k) { return x.host.querySelectorAll(".rvp-cfg")[2].querySelectorAll(".rvp-numb")[k]; }
var NOEV = { stopPropagation: function () {} };
assert.deepStrictEqual([text(numBtn(t, 0)), text(numBtn(t, 1))], ["-", "+"], "number = [-] value [+]");
numBtn(t, 1).onclick(NOEV); assert.strictEqual(cfgRows(t)[2], "4", "+ raises");
assert(!t.host.querySelectorAll(".rvp-edit").length, "a tap does not enter edit mode");
for (ci = 0; ci < 5; ci++) numBtn(t, 0).onclick(NOEV);
assert.strictEqual(cfgRows(t)[2], "1", "- lowers, clamped at min");
numBtn(t, 1).onclick(NOEV); numBtn(t, 1).onclick(NOEV); assert.strictEqual(cfgRows(t)[2], "3");
assert(text(t.host).indexOf("Accent colour") >= 0, "labels shown");
// ---- config_select_picker: Enter opens the option list, focus on the current one ----
assert(hasCls(foc(t), "rvp-cfg"), "first row focused");
t.key(13);
assert.deepStrictEqual(t.host.querySelectorAll(".rvp-optl").map(text), ["Red", "Green", "Blue"], "picker lists the options");
assert(text(t.host).indexOf("Accent colour") >= 0, "picker titled with the label");
assert(text(foc(t)).indexOf("Red") >= 0, "focus on the current value");
t.key(77); t.key(77); t.key(13);
assert.strictEqual(cfgRows(t)[0], "Blue", "picked");
assert(hasCls(foc(t), "rvp-cfg") && text(foc(t)).indexOf("Accent colour") >= 0, "focus back on the row");
t.key(13); assert(text(foc(t)).indexOf("Blue") >= 0, "reopened on the new value");
t.key(78); t.key(8);
assert.strictEqual(cfgRows(t)[0], "Blue", "Back keeps the value");
t.key(13); t.key(78); t.key(78); t.key(13);
assert.strictEqual(cfgRows(t)[0], "Red");
t.key(77); t.key(77); t.key(77); t.key(13); assert.strictEqual(cfgRows(t)[3], "Off", "switch toggles");
t.key(13); assert.strictEqual(cfgRows(t)[3], "On");
// ---- config_range_edit ----
t.key(78); t.key(78); t.key(13);
assert(hasCls(foc(t), "rvp-edit"), "edit mode on the size row");
for (var ci = 0; ci < 5; ci++) t.key(77);
assert.strictEqual(cfgRows(t)[1], "24", "clamped at max");
assert.strictEqual(rngFill(t), "100%", "slider follows the knob");
t.key(78); assert.strictEqual(cfgRows(t)[1], "22", "CCW steps down");
t.key(13); assert(!hasCls(foc(t), "rvp-edit"), "Enter leaves edit mode");
t.key(77); assert.strictEqual(cfgRows(t)[1], "22", "knob moves focus again");
t.key(13); for (ci = 0; ci < 9; ci++) t.key(77);
assert.strictEqual(cfgRows(t)[2], "9", "number clamps at max, steps by 1");
t.key(8); assert(!hasCls(foc(t), "rvp-edit") && hasCls(foc(t), "rvp-cfg"), "Back leaves edit mode first");
// ---- config_apply ----
t = mk(C([cfgItem()]));
openPage(t, "Skin"); btn(t, "Configuration").onclick();
assert(hasCls(btn(t, "Apply"), "dis"), "Apply disabled while unchanged");
t.key(13); t.key(77); t.key(13);   // accent -> Green
assert(!hasCls(btn(t, "Apply"), "dis"), "Apply enabled after a change");
btn(t, "Apply").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value],
  ["config-set", { name: "skin", values: { accent: "#2ECC71", size: 16, count: 3, clock: true } }]);
t.ctl.onDaemon({ t: "result", name: "skin", op: "config-set", ok: true, restart: false });
var ci2 = cfgItem(); ci2.config.values.accent = "#2ECC71";
t.ctl.onDaemon(C([ci2]));
assert(hasCls(btn(t, "Apply"), "dis"), "applied values = Apply disabled again");
assert.deepStrictEqual(cfgRows(t), ["Green", "16", "3", "On"], "still on the Config screen");
assert(text(t.host).indexOf("Settings applied") >= 0 && text(t.host).indexOf("HMI restart") < 0, "config result note, not the menu restart note");
t.ctl.onDaemon({ t: "result", name: "skin", op: "config-set", ok: false, error: "bad value" });
t.ctl.onDaemon(C([ci2]));
assert(text(t.host).indexOf("bad value") >= 0, "error shown on the Config screen");
// ---- config_back ----
t.key(8);
assert(btn(t, "Configuration") && text(t.host).indexOf("About") >= 0, "Back returns to the package page");
btn(t, "Configuration").onclick();
t.ctl.onDaemon(C([I("skin", { title: "Skin" })]));
assert(!t.host.querySelectorAll(".rvp-cfg").length, "uninstalled: Config screen left");

// ---- update_confirm_lists_versions ----------------------------------------------------------------
t = mk(C([I("t-basic", { state: "update", installed: "1.0.0", version: "2.0.0" })]));
openPage(t, "T t-basic"); btn(t, "Update").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["plan-update", { name: "t-basic" }]);
t.ctl.onDaemon({ t: "plan", name: "other", install: ["other"] });
assert(text(t.host).indexOf("These packages") < 0, "other client's plan ignored");
t.ctl.onDaemon({ t: "plan", name: "t-basic", install: ["t-basic"] });
assert(text(t.host).indexOf("t-basic 1.0.0 -> 2.0.0") >= 0, text(t.host));
assert.strictEqual(focText(t), "Update", "focus on OK");
t.key(13);
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["update", { name: "t-basic" }]);
assert(text(t.host).indexOf("These packages") < 0 && text(t.host).indexOf("About") >= 0, "back on the page");
// progress replaces the buttons, in place
t.ctl.onDaemon({ t: "progress", name: "t-basic", stage: "download", pct: 10 });
assert(text(t.host).indexOf("Downloading\u2026 10%") >= 0 && btns(t).indexOf("Update") < 0, "progress on the page");
t.ctl.onDaemon({ t: "progress", name: "t-basic", stage: "apply", pct: 40 });
var fill = t.host.querySelectorAll(".rvp-fill");
assert(fill.length === 1 && fill[0].style.width === "40%" && text(t.host).indexOf("Installing\u2026 40%") >= 0, "progress bar");
t.ctl.onDaemon({ t: "result", name: "t-basic", op: "update", ok: true, restart: false });
assert.strictEqual(text(t.host.querySelectorAll(".rvp-msg")[0]), "Menu changes appear after an HMI restart");
t.ctl.onDaemon(C([I("t-basic", { state: "installed", installed: "2.0.0", version: "2.0.0" })]));
assert.deepStrictEqual(btns(t), ["Remove"]);
assert.strictEqual(t.host.querySelectorAll(".rvp-bar").length, 0, "bar gone after result");
// install confirm, cancel = back to the page
t = mk(C([I("t-dep"), I("t-basic")]));
openPage(t, "T t-dep"); btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["plan", { name: "t-dep" }]);
t.ctl.onDaemon({ t: "plan", name: "t-dep", install: ["t-basic", "t-dep"] });
assert(text(t.host).indexOf("t-basic") >= 0 && btns(t).join() === "Install,Cancel", "install confirm");
t.key(77); assert.strictEqual(focText(t), "Cancel"); t.key(13);
assert(text(t.host).indexOf("About") >= 0 && lastStore(t).action === "plan", "cancel: page again, nothing sent");
// plan error -> on the page, no confirm; failed result keeps its error over the next catalog
btn(t, "Install").onclick();
t.ctl.onDaemon({ t: "plan", name: "t-dep", install: [], error: "t-dep conflicts with t-basic" });
assert(text(t.host).indexOf("conflicts with t-basic") >= 0 && btns(t).indexOf("Cancel") < 0);
t.ctl.onDaemon({ t: "result", name: "t-dep", op: "install", ok: false, error: "boom" });
t.ctl.onDaemon(C([I("t-dep"), I("t-basic")]));
assert(text(t.host).indexOf("boom") >= 0, "error survives the catalog that follows a result");
t.ctl.onDaemon(C([I("t-dep"), I("t-basic")]));
assert(text(t.host).indexOf("boom") < 0, "a later catalog clears it");
// ok + error = warning: shown, still a success
t.ctl.onDaemon({ t: "result", name: "t-dep", op: "install", ok: true, restart: false, error: "files were written; remount ro failed" });
assert(text(t.host.querySelectorAll(".rvp-msg")[0]).indexOf("files were written") === 0);
assert.strictEqual(modalText(t), "");
// plan arriving after the page was left: dropped with a note
btn(t, "Install").onclick(); t.key(8);
t.ctl.onDaemon({ t: "plan", name: "t-dep", install: ["t-basic", "t-dep"] });
assert(text(t.host).indexOf("These packages") < 0 && text(t.host).indexOf("not started") >= 0, "plan dropped with a note");
// error frame -> top line; offline -> No catalog yet
t.ctl.onDaemon({ t: "error", error: "busy" });
assert.strictEqual(text(t.host.querySelectorAll(".rvp-msg")[0]), "busy");
t.ctl.onDaemon({ t: "offline" });
assert(text(t.host).indexOf("No catalog yet") >= 0);
assert.strictEqual(t.host.querySelectorAll(".rvp-msg").length, 0, "offline clears the top line");

// ---- catalog_text_is_not_html ---------------------------------------------------------------------
var EVIL = "<b>x</b>&amp;";
t = mk(C([I("e1", { title: EVIL, subtitle: EVIL, description: EVIL })], { news: [{ date: "2026-11-14", title: EVIL, text: EVIL }] }));
assert(text(t.host).indexOf(EVIL) >= 0, "title verbatim on the card");
openPage(t, EVIL);
assert(text(t.host).split(EVIL).length - 1 >= 2, "title + description verbatim on the page");
t.key(8); section(t, "Home");
assert(text(t.host).split(EVIL).length - 1 >= 2, "news title + text verbatim");
assert.strictEqual(all(t.frame).concat(all(t.slot)).filter(function (n) { return n.tagName === "b"; }).length, 0, "no b element");

// ---- focus_scrolls_via_ctx ------------------------------------------------------------------------
t = mk(C([I("a1"), I("a2"), I("a3")]));
assert(hasCls(foc(t), "rvp-card"), "focus starts on the first card");
card(t, "T a2")._rect = { top: 500, bottom: 580 };
t.scrolls.length = 0;
t.key(77);
assert(text(foc(t)).indexOf("T a2") >= 0);
assert(t.scrolls.length === 1 && t.scrolls[0] > 0, "scrollTo called: " + t.scrolls);
assert.strictEqual(t.port.scrollTop, undefined, "the renderer never sets scrollTop with a ctx");
t.host.ev.click({ target: card(t, "T a3").children[1] });   // capture on the content, before the card's onclick
assert(text(foc(t)).indexOf("T a3") >= 0, "tap moves the selector onto the tapped card");

// ---- pinned_in_apps_locked_no_remove ----------------------------------------------------------
t = mk(C([I("cmu-re-store", { title: "Revised Store", category: "System mods", state: "installed", installed: "1.0.0", pinned: true, locked: true }),
  I("st", { category: "System mods", state: "installed", installed: "1.0.0", locked: true }), I("a1")]));
assert(card(t, "Revised Store") && card(t, "T a1"), "the store is listed in Apps");
assert(card(t, "Revised Store").querySelectorAll(".rvp-bi-info").length === 1 && text(card(t, "Revised Store")).indexOf("Installed") < 0, "installed: icon, no text");
openPage(t, "Revised Store"); assert.deepStrictEqual(btns(t), [], "store: no Remove"); t.key(8);
t.ctl.onDaemon(C([I("cmu-re-store", { title: "Revised Store", state: "update", installed: "0.9.0", pinned: true, locked: true }),
  I("st", { category: "System mods", state: "installed", installed: "1.0.0", locked: true })]));
openPage(t, "Revised Store"); assert.deepStrictEqual(btns(t), ["Update with the update stick"]); t.key(8);
section(t, "SysMods");
openPage(t, "T st"); assert.deepStrictEqual(btns(t), [], "locked: no Remove");

// ---- filter_chips --------------------------------------------------------------------------------
t = mk(C([I("a1", { state: "installed", installed: "1.0.0" }), I("a2"), I("a3", { state: "update", installed: "0.9.0" })]));
function chip(x, label) { return x.host.querySelectorAll(".rvp-chip").filter(function (b) { return text(b) === label; })[0]; }
function shown(x) { return x.host.querySelectorAll(".rvp-card").map(function (c) { return text(c.children[1]); }); }
assert.deepStrictEqual(shown(t), ["T a1", "T a2", "T a3"]);
chip(t, "Installed").onclick(); assert.deepStrictEqual(shown(t), ["T a1", "T a3"], "installed incl. updates");
assert(!hasCls(chip(t, "Installed"), "ghost") && hasCls(chip(t, "All"), "ghost"), "active chip highlighted");
chip(t, "Available").onclick(); assert.deepStrictEqual(shown(t), ["T a2"]);
chip(t, "All").onclick(); assert.strictEqual(shown(t).length, 3);

// ---- burger_glyph_after_pick ---------------------------------------------------------------------
t = mk(C([I("a1")]));
section(t, "SysMods");
assert(t.frame.querySelectorAll(".rvp-burger")[0]._html.indexOf("M128") < 0, "burger back to the menu glyph");

// ---- restart_modal_on_result ------------------------------------------------------------------
t = mk(C([I("a1", { title: "A" })]));
openPage(t, "A"); btn(t, "Install").onclick();
t.ctl.onDaemon({ t: "plan", name: "a1", install: ["a1"] }); t.key(13);
t.ctl.onDaemon({ t: "result", name: "a1", op: "install", ok: true, restart: false });
assert.strictEqual(modalText(t), "", "restart:false: no modal");
t.ctl.onDaemon({ t: "result", name: "a1", op: "install", ok: true, restart: true });
assert(modalText(t).indexOf("Restart required") >= 0, "modal on restart:true");
assert(modalText(t).indexOf("Restart the head unit to apply: A") >= 0, modalText(t));
assert.strictEqual(focText(t), "Restart", "focus on Restart");
t.key(13);
assert.strictEqual(lastStore(t).action, "reboot");
assert(modalText(t).indexOf("Restarting...") >= 0, "busy text");
assert.strictEqual(t.frame.querySelectorAll(".rvp-modal")[0].querySelectorAll(".rvp-btn").length, 0, "no buttons while busy");
t.ctl.onDaemon({ t: "rebooting" });
assert(modalText(t).indexOf("Restarting...") >= 0);
// I2: the header marker must not replace the busy modal (Later would reopen the store mid-shutdown)
t.ctl.onDaemon(C([I("a1", { title: "A", state: "installed", installed: "1.0.0" })], { restartPending: ["A"] }));
t.slot.querySelectorAll(".rvp-pend")[0].onclick();
assert(modalText(t).indexOf("Restarting...") >= 0, "marker keeps the busy modal");
assert.strictEqual(t.frame.querySelectorAll(".rvp-modal")[0].querySelectorAll(".rvp-btn").length, 0, "still no buttons");

// ---- restart_modal_error_restores_buttons ----------------------------------------------------------
t.ctl.onDaemon({ t: "error", error: "restart failed" });
assert(modalText(t).indexOf("restart failed") >= 0 && modalText(t).indexOf("Restarting...") < 0, modalText(t));
assert.deepStrictEqual(t.frame.querySelectorAll(".rvp-modal")[0].querySelectorAll(".rvp-btn").map(text), ["Later", "Restart"]);
assert.strictEqual(focText(t), "Restart");

// ---- restart_later_marks_header -------------------------------------------------------------------
t.ctl.onDaemon(C([I("a1", { title: "A", state: "installed", installed: "1.0.0" })], { restartPending: ["A"] }));
t.key(78); assert.strictEqual(focText(t), "Later"); t.key(13);
assert.strictEqual(modalText(t), "", "Later closes the modal");
var mark = t.slot.querySelectorAll(".rvp-pend");
assert(mark.length === 1 && text(mark[0]) === "Restart pending", "header marker");
mark[0].onclick();
assert(modalText(t).indexOf("Restart required") >= 0, "marker reopens the modal");
assert.strictEqual(t.key(8), true); assert.strictEqual(modalText(t), "", "key 8 = Later");
assert.strictEqual(t.slot.querySelectorAll(".rvp-pend").length, 1);
t = mk(); t.ctl.onDaemon(C([I("a1")], { restartPending: ["A"] }));
assert.strictEqual(t.slot.querySelectorAll(".rvp-pend").length, 1, "marker on a fresh render");

// ---- restart_blocks_exit ---------------------------------------------------------------------------
t = mk(C([I("a1")], { restartPending: ["A"] }));
t.key(8);   // the reopen modal (see below) closed with back
assert.strictEqual(modalText(t), "");
assert.strictEqual(t.key(8), true, "back at the root is consumed while a restart is pending");
assert(modalText(t).indexOf("Restart required") >= 0, "and opens the modal");
t.key(78); t.key(13); assert.strictEqual(modalText(t), "", "Later closes only the modal");
assert(card(t, "T a1"), "still in the store");
assert.strictEqual(t.key(8), true); assert.strictEqual(t.key(8), true);
assert.strictEqual(t.closed, 0, "ctx.close never called");
t = mk(C([I("a1")]));
assert.strictEqual(t.key(8), false, "empty list: back at the root goes to the SDK");

// ---- restart_modal_on_reopen ------------------------------------------------------------------------
t = mk(C([I("a1")], { restartPending: ["A"] }));
assert(modalText(t).indexOf("Restart required") >= 0, "pending at open: modal at once");
assert.strictEqual(focText(t), "Restart");
t.key(8); t.ctl.onDaemon(C([I("a1")], { restartPending: ["A"] }));
assert.strictEqual(modalText(t), "", "only the first catalog opens it");

// ---- restart_modal_lists_all_pending ----------------------------------------------------------------
t = mk(C([I("a1")], { restartPending: ["A", "B"] }));
assert(modalText(t).indexOf("Restart the head unit to apply: A, B") >= 0, modalText(t));

// ---- drawer: own focus list, back closes it -----------------------------------------------------------
t = mk(C([I("a1")]));
var bg = t.frame.querySelectorAll(".rvp-burger")[0];
assert(bg._html.indexOf("M128") < 0, "closed drawer: menu glyph");
bg.onclick();
assert(bg._html.indexOf("M128") >= 0, "open drawer: dots glyph on the burger");
assert.strictEqual(focText(t), "Apps", "drawer focus on the current section");
t.key(77); assert.strictEqual(focText(t), "SysMods");
assert.strictEqual(t.key(8), true, "back closes the drawer");
assert(bg._html.indexOf("M128") < 0, "back restores the menu glyph");
assert(hasCls(foc(t), "rvp-card"), "focus back in the list");

// ---- rating_stars_svg ----------------------------------------------------------------------------
t = mk(C([I("s1", { rating: { avg: 3.2, count: 7 }, everInstalled: true, myStars: 2 })]));
cd = card(t, "T s1");
assert(text(cd).indexOf("3.2") >= 0 && text(cd).indexOf("★") < 0, "number stays, no glyph");
assert.deepStrictEqual([stars(cd, "on"), stars(cd, "off")], [3, 2], "card: 3 filled, 2 empty");
openPage(t, "T s1");
var ph = t.host.querySelectorAll(".rvp-phead")[0];
assert(text(ph).indexOf("3.2") >= 0 && text(ph).indexOf("7 reviews") >= 0, text(ph));
assert.deepStrictEqual([stars(ph.querySelectorAll(".rvp-stv")[2], "on"), stars(ph.querySelectorAll(".rvp-stv")[2], "off")], [3, 2], "page: 3 filled, 2 empty next to the version");
var yr = ph.querySelectorAll(".rvp-rbtn")[0];
assert.deepStrictEqual([stars(yr, "on"), stars(yr, "off")], [2, 3], "Rate in the header shows own rating 2 of 5");
yr.onclick();
assert.strictEqual(filled(t), "11000", "rated 2: the picker opens on star 2");
assert(all(t.frame).concat(all(t.slot)).every(function (n) { return n.tagName !== "#text" || n._t.indexOf("★") < 0; }), "no star glyph anywhere");

// ---- home_order ---------------------------------------------------------------------------------
var NEWS = [{ date: "2026-11-01", title: "N old", text: "o" }, { date: "2026-11-10", title: "N new", text: "n", pkg: "f2" }];
var UNL = { linked: false, name: null };
t = mk(C([I("f1"), I("f2"), I("f3")], { featured: ["f3", "nope", "f1"], news: NEWS, account: UNL }));
section(t, "Home");
var h = text(t.host), bn = t.host.querySelectorAll(".rvp-banner");
assert.strictEqual(bn.length, 1, "unlinked: banner");
var ord = [text(bn[0]), "Featured", "T f3", "T f1", "News", "N new", "N old"].map(function (s) { return h.indexOf(s); });
assert(ord.every(function (v, i) { return v >= 0 && (i === 0 || v > ord[i - 1]); }), "home order " + ord);
assert(h.indexOf("T f2") < 0, "only featured packages are cards");
bn[0].onclick();
assert(t.host.querySelectorAll(".rvp-pair").length === 1 && lastStore(t).action === "pair-start", "banner opens Pair");
t.key(8);
t = mk(C([I("f1"), I("f2")], { featured: ["f1"], news: NEWS }));
section(t, "Home");
assert.strictEqual(t.host.querySelectorAll(".rvp-banner").length, 0, "linked: no banner");
t.host.querySelectorAll(".rvp-news").filter(function (n) { return text(n).indexOf("N new") >= 0; })[0].onclick();
assert(text(t.host).indexOf("T f2") >= 0 && text(t.host).indexOf("About") >= 0, "news pkg opens the page");
assert.strictEqual(t.titles[t.titles.length - 1], "Revised Store - Home");
t = mk(); section(t, "Home");
assert(text(t.host).indexOf("No catalog yet") >= 0);

// ---- account_states -----------------------------------------------------------------------------
t = mk(C([], { account: UNL, daemon: "0.1.0", store: "0.3.0" }));
global.window.RevisedHMI = { apiVersion: 1 };   // the SDK the store runs on
section(t, "Account");
delete global.window.RevisedHMI;
["Store v0.3.0", "SDK API 1", "Daemon v0.1.0"].forEach(function (v) { assert(text(t.host).indexOf(v) >= 0, v + " in " + text(t.host)); });
assert(!btn(t, "Sign out"), "unlinked: no Sign out");
btn(t, "Link this car").onclick();
assert(t.host.querySelectorAll(".rvp-pair").length === 1 && lastStore(t).action === "pair-start", "Link this car opens Pair");
t.key(8);
t.ctl.onDaemon(C([]));
assert(text(t.host).indexOf("Linked to Dev user") >= 0 && !btn(t, "Link this car"), text(t.host));
btn(t, "Sign out").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["signout", null]);

// ---- developer mode entry gate ------------------------------------------------------------------
var RISK = "Developer mode turns off the store's safety checks. Packages and settings you change here can leave the head unit unable to boot.";
var FACT = "Bricked units are not supported once Developer mode was on, even after you leave it. Support returns only after a full firmware reflash (.up) and a clean store install.";
var LOST = "Brick support: OFF (Developer mode was used). Reflash + clean install to restore.";
var REASON = "Link this car and go online first";
function acct(o) { var x = mk(C([], o)); section(x, "Account"); return x; }
function enableBtn(x) { return btn(x, "Enable Developer mode"); }
// devmode_enable_button
t = acct();
assert(enableBtn(t) && !hasCls(enableBtn(t), "dis") && text(t.host).indexOf(REASON) < 0, "enabled when linked + online");
t = acct({ account: { linked: false } });
assert(hasCls(enableBtn(t), "dis") && text(t.host).indexOf(REASON) >= 0, "unlinked: disabled + reason");
t = acct({ online: false });
assert(hasCls(enableBtn(t), "dis") && text(t.host).indexOf(REASON) >= 0, "offline: disabled + reason");
// devmode_modal_copy
t = acct(); enableBtn(t).onclick();
assert(modalText(t).indexOf("DEVELOPER MODE") >= 0 && modalText(t).indexOf(RISK) >= 0 && modalText(t).indexOf(FACT) >= 0, modalText(t));
assert(!(foc(t) && hasCls(foc(t), "rvp-dhandle")), "dragGate not knob-focusable");
// drag_gate_agree
function drag(x, pct) {
  var tr = x.frame.querySelectorAll(".rvp-dtrack")[0], h = x.frame.querySelectorAll(".rvp-dhandle")[0];
  tr.offsetWidth = 300; h.offsetWidth = 60;
  h.onmousedown({ clientX: 100, preventDefault: function () {} });
  docEv.mousemove({ clientX: 100 + 240 * pct }); docEv.mouseup({});
  assert(!docEv.mousemove && !docEv.mouseup, "document listeners removed");
  return h;
}
n0 = store(t).length;
var h = drag(t, 0.5);
assert.strictEqual(store(t).length, n0, "50%: nothing sent"); assert.strictEqual(h.style.left, "0px");
h = drag(t, 0.95);
assert.strictEqual(store(t).length, n0 + 1, "95%: one send"); assert.strictEqual(lastStore(t).action, "devmode-enable");
assert.strictEqual(modalText(t), "", "modal closed after agree");
// devmode_on_line
t = acct({ devmode: true });
assert(text(t.host).indexOf("Developer mode: ON") >= 0 && !enableBtn(t), "ON line, no enable button");
// support_lost_line
t = acct({ support: "lost" });
assert(text(t.host).indexOf(LOST) >= 0 && enableBtn(t), "lost line + enable button");
t = acct({ support: "ok" });
assert(text(t.host).indexOf(LOST) < 0, "ok: no lost line");

// ---- pair_screen --------------------------------------------------------------------------------
var QN = 37, ROWS = [], qr, qc;
for (qr = 0; qr < QN; qr++) { ROWS.push(""); for (qc = 0; qc < QN; qc++) ROWS[qr] += (qr * 7 + qc * 3) % 5 < 2 ? "1" : "0"; }
function qrEye(r, c) { return (r < 7 && c < 7) || (r < 7 && c >= QN - 7) || (r >= QN - 7 && c < 7); }
var QL = Math.floor(QN / 2) - 4, QH = Math.floor(QN / 2) + 4, darkBody = 0;
for (qr = 0; qr < QN; qr++) for (qc = 0; qc < QN; qc++)
  if (ROWS[qr].charAt(qc) === "1" && !qrEye(qr, qc) && !(qr >= QL && qr <= QH && qc >= QL && qc <= QH)) darkBody++;
function pairFrame(exp) { return { t: "pair", code: "aB3dE9xQ", url: "https://cmu-revised.com/c_id/aB3dE9xQ", expiresIn: exp, qr: ROWS }; }
function lastFrame(cv) {   // calls since the last background fill
  var c = cv.ctx2d.calls, i;
  for (i = c.length - 1; i >= 0; i--) if (c[i].f === "fillRect" && c[i].fs === "#0C0C0C") return c.slice(i);
  return [];
}
t = mk(C([], { account: UNL })); section(t, "Account");
btn(t, "Link this car").onclick();
t.ctl.onDaemon(pairFrame(600));
runClock(1500);
var cvs = all(t.host).filter(function (n) { return n.tagName === "canvas"; });
assert.strictEqual(cvs.length, 1, "one canvas");
var grads = cvs[0].ctx2d.calls.filter(function (c) { return c.f === "createLinearGradient"; });
assert.strictEqual(grads.length, 1, "one gradient");
assert.deepStrictEqual(grads[0].g.stops, [[0, "#FF2A2A"], [1, "#9E0000"]]);
var lf = lastFrame(cvs[0]);
assert(lf.length > 0 && lf[0].a[2] === 800, "final frame starts with the background");
var dots = lf.filter(function (c) { return c.f === "arc" && c.a[3] === 0 && c.a[4] === 7 && c.fs === grads[0].g; });
assert.strictEqual(dots.length, darkBody, "one dot per dark body module");
assert(lf.filter(function (c) { return c.f === "arc" && c.fs === "#3A0C0E"; }).length > 0, "decorative dots");
assert.strictEqual(lf.filter(function (c) { return c.f === "arc" && c.a[3] === -Math.PI / 2; }).length, 9, "three eyes = 9 rounded squares");
assert(lf.some(function (c) { return c.f === "fillText" && c.a[0] === "RE"; }), "logo");
var pt8 = text(t.host);
assert(pt8.indexOf("aB3dE9xQ") >= 0 && pt8.indexOf("cmu-revised.com/c_id/aB3dE9xQ") >= 0 && pt8.indexOf("Expires in") >= 0, pt8);
assert(clock.q.length > 0, "countdown running");
t.ctl.onDaemon({ t: "pair-done", name: "Dev user" });
assert(t.host.querySelectorAll(".rvp-pair").length === 0 && btn(t, "Link this car"), "pair-done pops back to Account");
assert.notStrictEqual(lastStore(t).action, "pair-cancel", "pair-done sends no cancel");
assert.strictEqual(clock.q.length, 0, "timers stopped");

// ---- pair_expiry_requests_new_code ----------------------------------------------------------------
function pairStarts(t) { return store(t).filter(function (e) { return e.action === "pair-start"; }).length; }
t = mk(C([], { account: UNL })); section(t, "Account");
btn(t, "Link this car").onclick();
t.ctl.onDaemon(pairFrame(2));
assert(text(t.host).indexOf("Expires in 0:02") >= 0, text(t.host));
assert.strictEqual(pairStarts(t), 1);
runClock(2000);
assert.strictEqual(pairStarts(t), 2, "expiry asks for a new code");
runClock(5000);
assert.strictEqual(pairStarts(t), 2, "asked once per code");

// ---- pair_back_cancels ----------------------------------------------------------------------------
t.ctl.onDaemon(pairFrame(600));
assert.strictEqual(t.key(8), true);
assert.strictEqual(lastStore(t).action, "pair-cancel");
assert(t.host.querySelectorAll(".rvp-pair").length === 0, "popped");
runClock(700000);
assert.strictEqual(pairStarts(t), 2, "no timer left after back");

// ---- usb_screen -----------------------------------------------------------------------------------
function U(name, o) {
  var u = { path: "/tmp/mnt/sda1/" + name + ".rvp", name: name, version: "1.2.0", title: name, author: "me", description: "usb " + name,
    depends: "", conflicts: "", unsigned: false, pinned: false }, k;
  for (k in o || {}) u[k] = o[k];
  return u;
}
var USB = [U("u-sig"), U("u-raw", { unsigned: true, version: "" }), U("cmu-re-store", { pinned: true })];
t = mk(C([], { account: UNL })); section(t, "Account");
btn(t, "Install from USB").onclick();
assert.strictEqual(lastStore(t).action, "usb-scan");
t.ctl.onDaemon({ t: "usb", items: USB });
assert(card(t, "u-sig") && card(t, "cmu-re-store"), "usb items listed");
assert.strictEqual(card(t, "u-raw").querySelectorAll(".rvp-bi-warn").length, 1, "unsigned: yellow triangle");
openPage(t, "u-raw");
assert.deepStrictEqual(btns(t), ["Install"]); assert(hasCls(btn(t, "Install"), "dis"), "unsigned: Install disabled");
assert(text(t.host).indexOf("UNSIGNED") >= 0); t.key(8);
openPage(t, "cmu-re-store");
assert.deepStrictEqual(btns(t), ["Install with the update stick"]);
assert(hasCls(btn(t, "Install with the update stick"), "dis"), "pinned: disabled"); t.key(8);
openPage(t, "u-sig");
assert.deepStrictEqual(btns(t), ["Install"], "signed: Install even when unlinked");
assert(!hasCls(btn(t, "Install"), "dis"));
assert(text(t.host).indexOf("Permissions: not declared") >= 0, text(t.host));
btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["plan-usb", { path: "/tmp/mnt/sda1/u-sig.rvp" }]);
t.ctl.onDaemon({ t: "plan", name: "u-sig", install: ["u-sig"] });
assert(text(t.host).indexOf("These packages") >= 0 && text(t.host).indexOf("Permissions: not declared") >= 0, "usb confirm");
btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["install-usb", { path: "/tmp/mnt/sda1/u-sig.rvp" }]);
t.ctl.onDaemon(C([], { account: UNL }));
assert(text(t.host).indexOf("usb u-sig") >= 0, "a catalog does not drop the USB page");
t.key(8); t.key(8);
assert(btn(t, "Install from USB"), "back to Account");

// ---- drag_gate_closed_mid_drag_no_agree --------------------------------------------------------------
t = acct(); enableBtn(t).onclick();
var gtr = t.frame.querySelectorAll(".rvp-dtrack")[0], gh = t.frame.querySelectorAll(".rvp-dhandle")[0];
gtr.offsetWidth = 300; gh.offsetWidth = 60;
gh.onmousedown({ clientX: 100, preventDefault: function () {} });
var gmv = docEv.mousemove, gup = docEv.mouseup;
t.key(8); assert.strictEqual(modalText(t), "", "modal closed mid-drag");
n0 = store(t).length;
gmv({ clientX: 100 + 240 * 0.95 }); gup({});
assert.strictEqual(store(t).length, n0, "closed modal: no agree");
assert(!docEv.mousemove && !docEv.mouseup, "document listeners removed");

// ---- developer section ------------------------------------------------------------------------------
var SSHK = "Make a key on your computer: ssh-keygen -t ecdsa -b 256 -f cmu_key";
var SSHC = "Copy cmu_key.pub to the USB stick root as .cmu-re_ssh-publickey";
var SSHW = "SSH gives root access to anyone holding the key, over the same WiFi network.";
var TWOFF = "Tripwire is OFF: if this package breaks boot, only a reflash recovers the unit.";
var LEAVE = "Brick support stays OFF. Only a full firmware reflash (.up) and a clean store install restore it.";
var EVER = "SSH was enabled: anyone with the key may have changed files the store does not track.";
var SSH_PKG9 = "cmu-re-dev-ssh";
var NOSIG ="NOT SIGNED. No guarantees. A bad package can brick the unit.";
function DEV(o) {
  var c = { devmode: true, tripwire: { nor: true, all: true }, ssh: { on: false, ip: null, key: true, ever: false } }, k;
  for (k in o || {}) c[k] = o[k];
  return c;
}
function dnames(x) { x.frame.querySelectorAll(".rvp-burger")[0].onclick(); var r = x.frame.querySelectorAll(".rvp-ditem").map(text); x.key(8); return r; }
function mroot(x) { return x.frame.querySelectorAll(".rvp-modal")[0]; }
function dev(cat) { var x = mk(cat); section(x, "Developer"); return x; }
// developer_drawer_only_in_devmode
t = mk(C([]));
assert.deepStrictEqual(dnames(t), ["Home", "Apps", "SysMods", "Skins", "Account"]);
t = dev(C([], DEV()));
assert.strictEqual(t.titles[t.titles.length - 1], "Revised Store - Developer");
assert.deepStrictEqual(dnames(t), ["Home", "Apps", "SysMods", "Skins", "Developer", "Account"]);
// developer_drawer_gone_after_leave
t.ctl.onDaemon(C([], { support: "lost" }));
assert.strictEqual(t.titles[t.titles.length - 1], "Revised Store - Account");
assert(enableBtn(t), "Account shown");
assert.deepStrictEqual(dnames(t), ["Home", "Apps", "SysMods", "Skins", "Account"]);
// tripwire_rows_and_gate: each switch is a toggle row (state = the toggle, no ON/OFF button)
function twr(x, label) { return x.host.querySelectorAll(".rvp-cfg").filter(function (r) { return text(r).indexOf(label) >= 0; })[0]; }
function twOn(r) { return hasCls(r.querySelectorAll(".rvp-sw")[0], "on"); }
t = dev(C([], DEV()));
assert(twOn(twr(t, "NOR flip")) && twOn(twr(t, "All recovery")) && text(t.host).indexOf("Takes effect after restart") >= 0, text(t.host));
assert(!btn(t, "NOR flip: ON") && hasCls(twr(t, "NOR flip"), "rvp-act"), "toggle rows, knob-focusable");
twr(t, "NOR flip").onclick();
assert(modalText(t).indexOf("Drag to I AGREE") >= 0 && t.frame.querySelectorAll(".rvp-dtrack").length === 1, "off: drag gate");
n0 = store(t).length;
drag(t, 0.95);
assert.strictEqual(store(t).length, n0 + 1);
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["tripwire", { switch: "nor", on: false }]);
assert.strictEqual(modalText(t), "", "closed after agree");
t.ctl.onDaemon(C([], DEV({ tripwire: { nor: false, all: true } })));
assert(!twOn(twr(t, "NOR flip")) && twOn(twr(t, "All recovery")), "off shown on the toggle");
twr(t, "NOR flip").onclick();
assert(modalText(t).length > 0 && t.frame.querySelectorAll(".rvp-dtrack").length === 0, "on: plain confirm, no gate");
btn(t, "Turn on", mroot(t)).onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["tripwire", { switch: "nor", on: true }]);
assert.strictEqual(modalText(t), "");
// nor_row_disabled_when_all_off
t = dev(C([], DEV({ tripwire: { nor: true, all: false } })));
assert(hasCls(twr(t, "NOR flip"), "dis") && !hasCls(twr(t, "NOR flip"), "rvp-act") && !twr(t, "NOR flip").onclick && !hasCls(twr(t, "All recovery"), "dis"), "NOR disabled while All is off");
t = dev(C([], DEV({ tripwire: null })));
assert(!twr(t, "All recovery") && text(t.host).indexOf("Takes effect after restart") < 0, "no tripwire block when null");
// ssh_enable_disabled_without_key
t = dev(C([], DEV({ ssh: { on: false, ip: null, key: false, ever: false } })));
assert(hasCls(btn(t, "Enable SSH"), "dis") && text(t.host).indexOf(SSHK) >= 0 && text(t.host).indexOf(SSHC) >= 0, text(t.host));
t.ctl.onDaemon(C([], DEV()));
assert(!hasCls(btn(t, "Enable SSH"), "dis"), "key on the stick: enabled");
btn(t, "Enable SSH").onclick();
assert(modalText(t).indexOf(SSHW) >= 0, modalText(t));
btn(t, "Enable SSH", mroot(t)).onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["ssh-enable", null]);
assert.strictEqual(modalText(t), "");
// ssh_connect_line_live
t = dev(C([], DEV({ ssh: { on: true, ip: "192.0.2.17", key: true, ever: true } })));
assert(text(t.host).indexOf("ssh -i cmu_key -p 36000 cmu@192.0.2.17") >= 0 && !btn(t, "Enable SSH"), text(t.host));
t.ctl.onDaemon(C([], DEV({ ssh: { on: true, ip: null, key: true, ever: true } })));
assert(text(t.host).indexOf("Not connected to WiFi") >= 0 && text(t.host).indexOf("cmu@") < 0, text(t.host));
btn(t, "Disable SSH").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["uninstall", { name: "cmu-re-dev-ssh" }]);
// dev_only_hidden_in_sections
t = mk(C([I("d-app", { devOnly: true }), I("d-mod", { devOnly: true, category: "System mods" }), I("d-skin", { devOnly: true, category: "Skins" }), I("a1")],
  DEV({ featured: ["d-app", "a1"] })));
["Home", "Apps", "SysMods", "Skins"].forEach(function (s) { section(t, s); assert(text(t.host).indexOf("T d-") < 0, s + ": " + text(t.host)); });
section(t, "Home"); assert(card(t, "T a1"), "other featured still shown");
// dev_only_listed_in_developer
t = dev(C([I("t-dev", { devOnly: true }), I(SSH_PKG9, { devOnly: true }), I("a1")], DEV()));
assert(text(t.host).indexOf("Developer packages") >= 0 && card(t, "T t-dev"), text(t.host));
assert(!card(t, "T " + SSH_PKG9) && !card(t, "T a1"), "SSH package and normal items not listed");
openPage(t, "T t-dev"); btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["plan", { name: "t-dev" }]);
t.ctl.onDaemon({ t: "plan", name: "t-dev", install: ["t-dev"] });
btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["install", { name: "t-dev" }]);
// dev_only_block_hidden_when_none
t = dev(C([I(SSH_PKG9, { devOnly: true }), I("a1")], DEV()));
assert(text(t.host).indexOf("Developer packages") < 0, "no devOnly besides SSH: block hidden");
// unsigned_consent_lists_overrides
var OV = [{ live: "/jci/x.js", type: "replace", reason: "menu hook" }];
t = dev(C([], DEV()));
btn(t, "Install from USB").onclick();
assert.strictEqual(lastStore(t).action, "usb-scan");
t.ctl.onDaemon({ t: "usb", items: [U("u-raw", { unsigned: true, permissions: ["audio"], overrides: OV })] });
openPage(t, "u-raw");
assert(!hasCls(btn(t, "Install"), "dis"), "dev mode: unsigned Install enabled");
btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["plan-usb", { path: "/tmp/mnt/sda1/u-raw.rvp", unsigned: true }]);
t.ctl.onDaemon({ t: "plan", name: "u-raw", install: ["u-raw"] });
var ct9 = text(t.host);
assert(ct9.indexOf(NOSIG) >= 0 && ct9.indexOf("Permissions: Audio") >= 0, ct9);
assert(ct9.indexOf("/jci/x.js") >= 0 && ct9.indexOf("replace") >= 0 && ct9.indexOf("menu hook") >= 0, ct9);
btn(t, "Install").onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["install-usb", { path: "/tmp/mnt/sda1/u-raw.rvp", unsigned: true }]);
// tripwire_off_warning_on_confirm
function confirmText(cat) {
  var x = mk(cat); openPage(x, "T a1"); btn(x, "Install").onclick();
  x.ctl.onDaemon({ t: "plan", name: "a1", install: ["a1"] });
  assert(text(x.host).indexOf("These packages") >= 0, "confirm shown");
  return text(x.host);
}
assert(confirmText(C([I("a1")], DEV({ tripwire: { nor: true, all: false } }))).indexOf(TWOFF) >= 0, "All off: warning");
assert(confirmText(C([I("a1")], DEV())).indexOf(TWOFF) < 0, "All on: no warning");
assert(confirmText(C([I("a1")])).indexOf(TWOFF) < 0, "no tripwire: no warning");
t = mk(C([I("a1", { state: "update", installed: "0.9.0" })], DEV({ tripwire: { nor: true, all: false } })));
openPage(t, "T a1"); btn(t, "Update").onclick();
t.ctl.onDaemon({ t: "plan", name: "a1", install: ["a1"] });
assert(text(t.host).indexOf(TWOFF) >= 0, "update confirm: warning");
// leave_modal_lists_and_warns
var LV = [I("u-raw", { state: "installed", installed: "1.0.0", unsigned: true }), I("u-avail", { unsigned: true }), I("a1", { state: "installed", installed: "1.0.0" }),
  I("t-dev", { state: "installed", installed: "1.0.0", devOnly: true }), I("t-dev2", { devOnly: true }),
  I(SSH_PKG9, { state: "installed", installed: "1.0.0", devOnly: true })];
t = dev(C(LV, DEV({ tripwire: { nor: false, all: false }, ssh: { on: true, ip: null, key: true, ever: true } })));
btn(t, "Leave Developer mode").onclick();
assert.deepStrictEqual(mroot(t).querySelectorAll(".rvp-li").map(text),
  ["SSH (cmu-re-dev-ssh)", "Unsigned package: u-raw", "Developer package: T t-dev", "Tripwire All recovery: back ON", "Tripwire NOR flip: back ON"]);
assert(modalText(t).indexOf(LEAVE) >= 0 && modalText(t).indexOf(EVER) >= 0, modalText(t));
assert.strictEqual(t.frame.querySelectorAll(".rvp-dtrack").length, 0, "no drag gate");
btn(t, "Leave", mroot(t)).onclick();
assert.deepStrictEqual([lastStore(t).action, lastStore(t).value], ["devmode-leave", null]);
t.ctl.onDaemon({ t: "result", name: "devmode", op: "devmode-leave", ok: true, restart: true });
assert(modalText(t).indexOf("Restart required") >= 0, "restart:true: SafeReboot offer");
t.ctl.onDaemon(C(LV, { support: "lost", restartPending: ["Tripwire"] }));
assert.strictEqual(t.titles[t.titles.length - 1], "Revised Store - Account");
assert(modalText(t).indexOf("Restart the head unit to apply: Tripwire") >= 0, modalText(t));
t = dev(C([], DEV()));
btn(t, "Leave Developer mode").onclick();
assert(modalText(t).indexOf(LEAVE) >= 0 && modalText(t).indexOf(EVER) < 0, "no ssh.ever: no SSH line");
// leave_error_shown
btn(t, "Leave", mroot(t)).onclick();
t.ctl.onDaemon({ t: "error", error: "leave failed at ssh: boom" });
t.ctl.onDaemon(C([], DEV()));
assert(modalText(t).indexOf("leave failed at ssh: boom") >= 0, modalText(t));
assert.deepStrictEqual(mroot(t).querySelectorAll(".rvp-btn").map(text), ["Cancel", "Leave"]);
btn(t, "Leave", mroot(t)).onclick();
t.ctl.onDaemon({ t: "result", name: "devmode", op: "devmode-leave", ok: true, restart: false });
assert.strictEqual(modalText(t), "", "restart:false: modal closed, no offer");

// ---- SDK hosting: no ctx (desktop webapp) ---------------------------------------------------------------
listeners = {};
var f3 = new Node("div"), p3 = f3.appendChild(new Node("div")), h3 = p3.appendChild(new Node("div")), s3 = [];
var ctl3 = App.render(h3, function (e) { s3.push(e); });
assert(listeners.keydown && listeners.keyup, "default path binds the window key listeners");
ctl3.onDaemon(C([I("a1"), I("a2")]));
h3.querySelectorAll(".rvp-card")[1]._rect = { top: 600, bottom: 680 };
listeners.keydown({ keyCode: 77 });
assert(p3.scrollTop > 0, "no ctx: focus scroll sets the host parent's scrollTop");
listeners.keydown({ keyCode: 8 });
assert(s3.some(function (e) { return e.control === "back"; }), "no ctx: back at the root fires back");
ctl3.teardown();
listeners = {};
var f2 = new Node("div"), p2 = f2.appendChild(new Node("div")), h2 = p2.appendChild(new Node("div"));
var ctl2 = App.render(h2, function () {}, { bindKeys: false });
assert(!listeners.keydown && !listeners.keyup, "bindKeys:false adds no keydown/keyup listener");
ctl2.onDaemon(C([I("a1"), I("a2")]));
assert.strictEqual(ctl2.key({ keyCode: 77 }), true, "key 77 handled");
assert.strictEqual(ctl2.key({ keyCode: 65 }), false, "unknown key not handled");
assert.strictEqual(ctl2.keyUp({ keyCode: 77 }), true, "knob keyUp handled");
ctl2.teardown();

// ---- shim: fresh load of controls.js with a stub RevisedHMI --------------------------------------------
var regs = {};
var fakeBack = { style: {} };
global.window.RevisedHMI = { register: function (k, a) { regs[k] = a; } };
global.window.__rvpDaemon = null;
var realGet = document.getElementById;
document.getElementById = function (id) { return id === "rvp-sdk-back" ? fakeBack : null; };
delete require.cache[SRC];
require(SRC);
assert(regs["cmu-re-store/store"] && typeof regs["cmu-re-store/store"].open === "function", "shim registers cmu-re-store/store");
var dsent = [], dframes = [], offCalls = 0;
global.window.__rvpDaemon = {
  send: function (e) { dsent.push(e); },
  onFrame: function (fn) { dframes.push(fn); return function () { offCalls++; }; },
  last: function () { return C([I("a1")]); }
};
listeners = {};
var sh = mk(); // ctx stub only; the shim renders into its root
sh.host.children = [];
var inst = regs["cmu-re-store/store"].open(sh.ctx);
assert.notStrictEqual(fakeBack.style.display, "none", "shim leaves the SDK back button (header back = key 8)");
assert(!listeners.keydown, "shim adds no window key listener");
assert(!(inst && (inst.dragStart || inst.dragTo || inst.dragStop)), "no drag hooks: the SDK owns scrolling");
assert(text(sh.host).indexOf("T a1") >= 0, "last() catalog replayed into onDaemon");
assert(dsent.some(function (e) { return e.control === "store" && e.action === "catalog"; }), "shim asks the daemon for a fresh catalog");
assert.strictEqual(dframes.length, 1, "subscribed to daemon frames");
dframes[0]({ t: "error", error: "boom" });
assert.strictEqual(text(sh.host.querySelectorAll(".rvp-msg")[0]), "boom", "daemon frames reach the renderer");
assert.strictEqual(sh.keyFn({ key: 77, phase: "up" }), true, "up key handled");
assert.strictEqual(sh.keyFn({ key: 65, phase: "down" }), false, "unhandled key reported");
assert.strictEqual(sh.keyFn({ key: 8, phase: "down" }), false, "back at the root: SDK closes");
regs["cmu-re-store/store"].close();
assert.strictEqual(offCalls, 1, "close unsubscribes from daemon frames");
document.getElementById = realGet;
// offline_connect_help: never linked + server unreachable -> root screens show the Wi-Fi steps
t = mk(C(three, { online: false }));
assert(text(t.host).indexOf("Please connect CMU") < 0, "linked unit: no help (it has been online before)");
// offline_pill: offline store = header pill like "Restart pending"; tap shows the Wi-Fi steps in a modal
var op = t.slot.querySelectorAll(".rvp-pend");
assert(op.length === 1 && text(op[0]) === "Connect to the internet", text(t.slot));
op[0].onclick();
assert(modalText(t).indexOf("Devices") >= 0 && modalText(t).indexOf("next to the clock") >= 0, modalText(t));
t.key(13); assert.strictEqual(modalText(t), "", "Close closes the instructions");
t = mk(C(three, { online: true }));
assert(text(t.slot).indexOf("Connect to the internet") < 0, "no pill when online");
t = mk(C(three, { online: false }));
t = mk(C(three, { online: false, account: { linked: false } }));
assert(text(t.host).indexOf("Please connect CMU to the internet") >= 0 && text(t.host).indexOf("Devices") >= 0, text(t.host));
assert.strictEqual(t.host.querySelectorAll(".rvp-offs").length, 5, "five steps");
assert(text(t.host).indexOf("next to the clock") >= 0, "says where the online status shows");
t.ctl.onDaemon(C(three, { online: true, account: { linked: false } }));
assert(text(t.host).indexOf("Please connect CMU") < 0, "gone when online");
console.log("ALL OK");
