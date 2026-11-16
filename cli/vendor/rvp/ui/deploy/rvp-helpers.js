// DEV ONLY. Store-UI helpers for remote tests over the eval bridge.
// Load into the HMI page: `sh u.sh helpers` (lost on reboot; load again).
// ES5 only (Presto). click() dispatches mousedown/mouseup/click like a finger tap,
// so the injector's drag tracking and the renderer's tap focus sync see a real tap.
// Screen stack (T7/T8): drawer section -> list/home/account -> package page -> confirm / Pair / USB.
// Careful: press("Install") on a confirm pane INSTALLS.
window.__rvp = {
  txt: function (n) { return n ? (n.textContent || "") : ""; },
  q: function (s) { return document.querySelectorAll(s); },
  // "title | subtitle | badge" per card on the current screen
  cards: function () { var c = this.q(".rvp-card"), o = [], i, b; for (i = 0; i < c.length; i++) { b = c[i].querySelectorAll(".rvp-badge")[0]; o.push(this.txt(c[i].querySelectorAll(".rvp-nm")[0]) + " | " + this.txt(c[i].querySelectorAll(".rvp-sub")[0]) + " | " + (b ? this.txt(b) : "")); } return o; },
  // finger tap = mousedown, mouseup, click (the embedded Opera delivers touch as mouse events)
  click: function (el) { var t = ["mousedown", "mouseup", "click"], i, e; for (i = 0; i < 3; i++) { e = document.createEvent("MouseEvents"); e.initMouseEvent(t[i], true, true, window, 1, 0, 0, 0, 0, false, false, false, false, 0, null); el.dispatchEvent(e); } },
  byText: function (sel, t) { var a = this.q(sel), i; for (i = 0; i < a.length; i++) if (this.txt(a[i]).indexOf(t) >= 0) return a[i]; return null; },
  // exact text match (visible buttons only: the closed modal keeps none)
  exact: function (sel, t) { var a = this.q(sel), i; for (i = 0; i < a.length; i++) if (this.txt(a[i]) === t && a[i].offsetWidth > 0) return a[i]; return null; },
  // what is on screen: header title, then cards or the buttons
  screen: function () { var h = this.txt(document.getElementById("rvp-inj-title")) || document.title, c = this.cards(); return h + " :: " + (c.length ? c.join(" ; ") : this.buttons().join(" ; ")); },
  buttons: function () { var a = this.q(".rvp-btn"), o = [], i; for (i = 0; i < a.length; i++) if (a[i].offsetWidth > 0) o.push(this.txt(a[i]) + (a[i].className.indexOf(" dis") >= 0 ? " (disabled)" : "")); return o; },
  // drawer section: go("Home" | "Apps" | "SysMods" | "Skins" | "Account")
  go: function (sec) { var b = this.q(".rvp-burger")[0], d; if (!b) return "no burger"; this.click(b); d = this.exact(".rvp-ditem", sec); if (!d) return "no drawer item " + sec; this.click(d); return this.screen(); },
  // open the package page of the card whose text contains title
  open: function (title) { var c = this.byText(".rvp-card", title); if (!c) return "no card " + title; this.click(c); return this.screen(); },
  // press a button by its exact label (page, confirm, Account, modal), e.g. press("Install")
  press: function (label) { var b = this.exact(".rvp-btn", label); if (!b) return "no button " + label; this.click(b); return "pressed " + label; },
  // open title's page and press label on it, e.g. act("T basic", "Install") -> confirm pane
  act: function (title, label) { var r = this.open(title); if (r.indexOf("no card") === 0) return r; return this.press(label || "Install"); },
  // press OK on the confirm pane (Install / Update)
  confirm: function () { var p = this.q(".rvp-pane")[0], b = p ? this.exact(".rvp-pane .rvp-btn", "Install") || this.exact(".rvp-pane .rvp-btn", "Update") : null; if (!b) return "no confirm"; this.click(b); return "confirmed"; },
  // header back (= key 8: closes modal / drawer, pops the stack)
  back: function () { var b = document.getElementById("rvp-sdk-back") || this.q(".rvp-hback")[0]; if (!b) return "no back"; this.click(b); return this.screen(); },
  usb: function () { this.go("Account"); this.press("Install from USB"); return this.screen(); },
  pair: function () { return this.txt(this.q(".rvp-pcode")[0]) + " | " + this.txt(this.q(".rvp-purl")[0]) + " | " + this.txt(this.q(".rvp-pexp")[0]); },
  modal: function () { var m = this.q(".rvp-modal")[0]; return m && m.className.indexOf("rvp-on") >= 0 ? this.txt(m) : ""; },
  msgs: function () { var a = this.q(".rvp-msg, .rvp-err"), o = [], i; for (i = 0; i < a.length; i++) o.push(this.txt(a[i])); return o; },
  // where the knob selector is: "<control text> @ <card or zone text>"
  focus: function () { var f = this.q(".rvp-focus")[0]; return f ? this.txt(f) + " @ " + this.txt(f.parentNode).substring(0, 60) : "none"; }
};
"helpers ok";
