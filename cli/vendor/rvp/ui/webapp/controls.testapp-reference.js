/* Revised Store — test-app control renderer (ES5, Opera/Presto-safe).
 *
 * ONE source of truth for the test app's markup, styles and event shape.
 * Both hosts use it:
 *   - webapp/index.html  (offline demo in the browser pane)
 *   - deploy/developer_mode_GuiConfig.js  (on-unit Tier-A overlay)
 *
 * No ES6: var only, function expressions, no template literals, no Promise.
 * Everything is scoped under `.rvp-testapp` so injecting it into the live HMI
 * never touches stock styles.
 *
 * API (global RevisedTestApp):
 *   RevisedTestApp.injectStyles()          -> adds the <style> once (idempotent)
 *   RevisedTestApp.render(container, send)  -> builds the UI into `container`
 *       send(evt) is called on every interaction with:
 *           { control, action, value, ts }
 */
var RevisedTestApp = (function () {
  "use strict";

  var STYLE_ID = "rvp-testapp-styles";

  // Presto-safe styling, STATIC 2012-style layout — fixed 800x480 window, so
  // NO flexbox at all. The unit's embedded Opera for Devices SDK build renders
  // `display:flex` differently from desktop (flex children don't size, rows
  // stack) — desktop Presto hides this, so flex is simply banned. We lay out
  // with what the embedded build renders identically to desktop (proven on the
  // unit): inline-block + vertical-align, position:absolute for right-aligned
  // bits, and the font-size:0 + width:25% trick for equal tabs. Also:
  //   - NO css var() (dropped on every Presto) -> literal hex/rgba.
  //   - gradients written 3x, last-valid-wins: solid fallback (never white) ->
  //     -o-linear-gradient (old Presto) -> unprefixed linear-gradient (modern).
  // box-shadow / text-shadow / transform / border-radius / outline+glow render.
  var CSS =
    ".rvp-testapp{font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;background:#000;color:#E0E0E0;box-sizing:border-box;}" +
    ".rvp-testapp *{box-sizing:border-box;margin:0;padding:0;}" +
    /* header — inline-block + vertical-align, no flex */
    ".rvp-head{height:54px;line-height:54px;padding:0 16px;border-bottom:2px solid #2A2C33;white-space:nowrap;}" +
    ".rvp-back{display:inline-block;vertical-align:middle;width:34px;height:44px;background:#3A3A3A;background:-o-linear-gradient(top,#666,#222);background:linear-gradient(135deg,#666,#222);border:2px solid #888;border-radius:4px;box-shadow:0 4px 10px rgba(0,0,0,.5),inset 0 1px 2px rgba(255,255,255,.3);cursor:pointer;-webkit-clip-path:polygon(100% 0,100% 100%,20% 50%);clip-path:polygon(100% 0,100% 100%,20% 50%);}" +
    ".rvp-title{display:inline-block;vertical-align:middle;line-height:normal;font-size:22px;color:#F0F0F0;letter-spacing:.5px;margin-left:14px;text-shadow:0 2px 4px rgba(0,0,0,.8);}" +
    ".rvp-body{padding:20px;}" +
    ".rvp-group{margin-bottom:26px;}" +
    ".rvp-group-label{font-size:13px;font-weight:600;color:#8A94A6;text-transform:uppercase;letter-spacing:1px;margin-bottom:10px;}" +
    /* buttons — inline-block, no flex */
    ".rvp-btn{display:inline-block;vertical-align:top;padding:12px 24px;margin:0 16px 10px 0;font-size:14px;font-weight:600;text-transform:uppercase;border-radius:4px;cursor:pointer;border:1px solid #000;color:#FFF;background:#333;box-shadow:inset 0 1px 1px rgba(255,255,255,.2),0 4px 6px rgba(0,0,0,.6);}" +
    ".rvp-btn:active{-o-transform:translateY(2px);transform:translateY(2px);box-shadow:inset 0 2px 4px rgba(0,0,0,.8);}" +
    ".rvp-btn.primary{background:#B50E13;background:-o-linear-gradient(top,#E11218,#900B0F);background:linear-gradient(to bottom,#E11218,#900B0F);border:1px solid #F03A3F;}" +
    ".rvp-btn.secondary{background:#34363E;background:-o-linear-gradient(top,#4A4D56,#1C1D21);background:linear-gradient(to bottom,#4A4D56,#1C1D21);border:1px solid #6A6D76;}" +
    ".rvp-btn.warning{background:#C06F0A;background:-o-linear-gradient(top,#F59E0B,#92400E);background:linear-gradient(to bottom,#F59E0B,#92400E);border:1px solid #FCD34D;}" +
    ".rvp-btn.critical{background:#6E1414;background:-o-linear-gradient(top,#991B1B,#450A0A);background:linear-gradient(to bottom,#991B1B,#450A0A);border:1px solid #DC2626;font-weight:800;letter-spacing:1px;}" +
    /* list + checkbox — text in flow, checkbox absolute on the right */
    ".rvp-list{background:#000;border:1px solid #2A2C33;border-radius:4px;overflow:hidden;}" +
    ".rvp-item{position:relative;min-height:66px;padding:16px 72px 16px 24px;border-bottom:1px solid #2A2C33;cursor:pointer;border-left:4px solid transparent;}" +
    ".rvp-item:last-child{border-bottom:none;}" +
    /* hover highlight gated behind :not(.rvp-dragging): the injector sets that
       class on the content for the whole pointer-drag, so while scrolling the
       :hover selector matches nothing -> no per-row restyle/repaint as the
       finger passes over (the mig + lag). .focus (knob) always applies. */
    ".rvp-item.focus,.rvp-testapp:not(.rvp-dragging) .rvp-item:hover{background:rgba(225,18,24,0.22);background:-o-linear-gradient(left,rgba(225,18,24,0.7),rgba(0,0,0,0));background:linear-gradient(90deg,rgba(225,18,24,0.7) 0%,rgba(0,0,0,0) 80%);border-left:4px solid #E11218;}" +
    ".rvp-item-title{font-size:20px;color:#FFF;text-shadow:0 2px 4px rgba(0,0,0,.8);}" +
    ".rvp-item-sub{font-size:14px;color:#8A94A6;margin-top:4px;}" +
    ".rvp-cbx{position:absolute;right:24px;top:50%;margin-top:-16px;width:32px;height:32px;border:2px solid #555;border-radius:2px;background:#222;background:-o-linear-gradient(top,#333,#111);background:linear-gradient(to bottom,#333,#111);box-shadow:inset 0 1px 3px rgba(255,255,255,.2),0 2px 5px rgba(0,0,0,.8);}" +
    ".rvp-cbx.checked:after{content:'';position:absolute;left:9px;top:3px;width:10px;height:18px;border-right:3px solid #FFF;border-bottom:3px solid #FFF;-webkit-transform:rotate(45deg);-o-transform:rotate(45deg);transform:rotate(45deg);}" +
    /* radio — inline-block + vertical-align, no flex */
    ".rvp-radio-row{cursor:pointer;margin-bottom:16px;white-space:nowrap;}" +
    ".rvp-radio{display:inline-block;vertical-align:middle;position:relative;width:30px;height:30px;border:2px solid #555;border-radius:50%;background:#222;background:-o-linear-gradient(top,#333,#111);background:linear-gradient(to bottom,#333,#111);box-shadow:inset 0 1px 3px rgba(255,255,255,.2),0 2px 5px rgba(0,0,0,.8);margin-right:15px;}" +
    ".rvp-radio.checked:after{content:'';position:absolute;left:6px;top:6px;width:14px;height:14px;background:#E11218;border-radius:50%;box-shadow:0 0 8px #E11218;}" +
    ".rvp-radio-label{display:inline-block;vertical-align:middle;font-size:16px;color:#FFF;}" +
    /* accordion — icon absolute on the right, no flex */
    ".rvp-acc{border:1px solid #2A2C33;border-radius:4px;overflow:hidden;}" +
    ".rvp-acc-item{border-bottom:1px solid #2A2C33;}" +
    ".rvp-acc-item:last-child{border-bottom:none;}" +
    ".rvp-acc-head{position:relative;display:block;width:100%;padding:16px 44px 16px 20px;background:#24262B;background:-o-linear-gradient(top,#2A2C33,#1C1D21);background:linear-gradient(to bottom,#2A2C33,#1C1D21);color:#FFF;font-size:16px;font-weight:500;text-align:left;border:none;cursor:pointer;}" +
    ".rvp-acc-item.active .rvp-acc-head{border-left:4px solid #E11218;padding-left:16px;}" +
    ".rvp-acc-icon{position:absolute;right:20px;top:50%;margin-top:-7px;font-size:12px;line-height:12px;}" +
    ".rvp-acc-item.active .rvp-acc-icon{-webkit-transform:rotate(180deg);-o-transform:rotate(180deg);transform:rotate(180deg);}" +
    ".rvp-acc-body{max-height:0;overflow:hidden;-webkit-transition:max-height .3s ease;-o-transition:max-height .3s ease;transition:max-height .3s ease;background:#000;}" +
    ".rvp-acc-content{padding:20px;color:#8A94A6;line-height:1.6;font-size:14px;}" +
    /* tabs — font-size:0 parent + inline-block 25% children (no flex) */
    ".rvp-tabs-wrap{border:1px solid #2A2C33;border-radius:6px;overflow:hidden;}" +
    ".rvp-tab-content{min-height:90px;line-height:42px;padding:24px;font-size:20px;color:#FFF;text-align:center;}" +
    ".rvp-tabs{height:64px;background:#121316;border-top:1px solid #2A2C33;font-size:0;}" +
    ".rvp-tab-btn{display:inline-block;vertical-align:top;width:25%;height:64px;line-height:58px;background:#121316;border:none;border-top:3px solid transparent;color:#8A94A6;font-size:14px;font-weight:600;text-transform:uppercase;cursor:pointer;text-align:center;}" +
    ".rvp-tab-btn.active{color:#FFF;border-top:3px solid #E11218;background:rgba(225,18,24,.15);background:-o-linear-gradient(top,rgba(225,18,24,.15),rgba(0,0,0,0));background:linear-gradient(to bottom,rgba(225,18,24,.15) 0%,rgba(0,0,0,0) 100%);}" +
    /* commander-knob focus ring — literal colors so Presto actually paints it */
    ".rvp-focus{outline:4px solid #FF2A30;outline-offset:-4px;box-shadow:0 0 16px rgba(225,18,24,0.9);position:relative;z-index:1;}" +
    /* while dragging (rvp-fast) kill shadows/glow so each frame rasterizes cheap,
       like stock's flat list rows — the heavy per-element paint was the lag */
    ".rvp-fast *{box-shadow:none !important;text-shadow:none !important;}";

  function injectStyles() {
    if (document.getElementById(STYLE_ID)) return;
    var s = document.createElement("style");
    s.id = STYLE_ID;
    s.type = "text/css";
    s.appendChild(document.createTextNode(CSS));
    (document.head || document.getElementsByTagName("head")[0] || document.body).appendChild(s);
  }

  // DOM helper
  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.appendChild(document.createTextNode(text));
    return e;
  }

  function group(parent, label) {
    var g = el("div", "rvp-group");
    g.appendChild(el("div", "rvp-group-label", label));
    parent.appendChild(g);
    return g;
  }

  function render(container, send) {
    var emit = typeof send === "function" ? send : function () {};
    function fire(control, action, value) {
      emit({ control: control, action: action, value: value, ts: +new Date() });
    }

    container.className = (container.className ? container.className + " " : "") + "rvp-testapp";
    container.innerHTML = "";

    // --- header with curved back button ---
    var head = el("div", "rvp-head");
    var back = el("div", "rvp-back");
    back.title = "Back";
    back.onclick = function () { fire("back", "press", null); };
    head.appendChild(back);
    head.appendChild(el("div", "rvp-title", "Revised Store — Test App"));
    container.appendChild(head);

    var body = el("div", "rvp-body");
    container.appendChild(body);

    // --- buttons ---
    (function () {
      var g = group(body, "Buttons");
      var row = el("div", "rvp-btn-row");
      var defs = [
        ["primary", "Primary Action"],
        ["secondary", "Secondary"],
        ["warning", "Warning"],
        ["critical", "Critical System"]
      ];
      for (var i = 0; i < defs.length; i++) {
        (function (variant, label) {
          var b = el("button", "rvp-btn " + variant, label);
          b.onclick = function () { fire("button", "press", variant); };
          row.appendChild(b);
        })(defs[i][0], defs[i][1]);
      }
      g.appendChild(row);
    })();

    // --- list + checkboxes ---
    (function () {
      var g = group(body, "List & Checkboxes");
      var list = el("div", "rvp-list");
      var items = [
        ["touch_in_motion", "Touchscreen in Motion", "Enable touch inputs above 0 km/h", true],
        ["aa_diag", "Android Auto Diagnostics", "Show debug overlay on screen", false]
      ];
      for (var i = 0; i < items.length; i++) {
        (function (id, title, sub, checked) {
          var item = el("div", "rvp-item");
          var left = el("div");
          left.appendChild(el("div", "rvp-item-title", title));
          left.appendChild(el("div", "rvp-item-sub", sub));
          var cbx = el("div", "rvp-cbx" + (checked ? " checked" : ""));
          item.appendChild(left);
          item.appendChild(cbx);
          var state = checked;
          item.onclick = function () {
            state = !state;
            if (state) cbx.className = "rvp-cbx checked";
            else cbx.className = "rvp-cbx";
            fire("checkbox", "toggle", { id: id, checked: state });
          };
          list.appendChild(item);
        })(items[i][0], items[i][1], items[i][2], items[i][3]);
      }
      g.appendChild(list);
    })();

    // --- radio group ---
    (function () {
      var g = group(body, "Radio Buttons");
      var wrap = el("div", "rvp-radio-group");
      var opts = [
        ["standard", "Standard Operating Mode", true],
        ["developer", "Developer / Debug Mode", false]
      ];
      var boxes = [];
      for (var i = 0; i < opts.length; i++) {
        (function (val, label, checked, idx) {
          var row = el("div", "rvp-radio-row");
          var box = el("div", "rvp-radio" + (checked ? " checked" : ""));
          boxes[idx] = box;
          row.appendChild(box);
          row.appendChild(el("span", "rvp-radio-label", label));
          row.onclick = function () {
            for (var j = 0; j < boxes.length; j++) boxes[j].className = "rvp-radio";
            box.className = "rvp-radio checked";
            fire("radio", "select", val);
          };
          wrap.appendChild(row);
        })(opts[i][0], opts[i][1], opts[i][2], i);
      }
      g.appendChild(wrap);
    })();

    // --- accordion ---
    (function () {
      var g = group(body, "Accordion");
      var acc = el("div", "rvp-acc");
      var defs = [
        ["daemon_config", "Daemon Configuration", "Configure the aa-touchbridged daemon: polling rates, input event handling and memory constraints for the CMU150 hardware limits."],
        ["safety_recovery", "Safety & Recovery", "Automatic rootfs backups are created before any system modification. Restore to factory OEM state here in case of boot loops."]
      ];
      for (var i = 0; i < defs.length; i++) {
        (function (id, title, text) {
          var item = el("div", "rvp-acc-item");
          var headBtn = el("button", "rvp-acc-head");
          headBtn.appendChild(document.createTextNode(title));
          headBtn.appendChild(el("span", "rvp-acc-icon", "▼"));
          var bodyEl = el("div", "rvp-acc-body");
          var content = el("div", "rvp-acc-content", text);
          bodyEl.appendChild(content);
          headBtn.onclick = function () {
            var open = item.className.indexOf("active") === -1;
            if (open) {
              item.className = "rvp-acc-item active";
              bodyEl.style.maxHeight = content.offsetHeight + 40 + "px";
            } else {
              item.className = "rvp-acc-item";
              bodyEl.style.maxHeight = "0";
            }
            fire("accordion", open ? "expand" : "collapse", id);
          };
          item.appendChild(headBtn);
          item.appendChild(bodyEl);
          acc.appendChild(item);
        })(defs[i][0], defs[i][1], defs[i][2]);
      }
      g.appendChild(acc);
    })();

    // --- bottom tabs ---
    (function () {
      var g = group(body, "Bottom Tabs");
      var wrap = el("div", "rvp-tabs-wrap");
      var content = el("div", "rvp-tab-content", "Audio & Entertainment Settings");
      var tabs = el("div", "rvp-tabs");
      var defs = [
        ["audio", "Audio", "Audio & Entertainment Settings"],
        ["nav", "Nav", "Navigation Map Config"],
        ["system", "System", "Core System Modifications"],
        ["logs", "Logs", "Live System Terminal"]
      ];
      var btns = [];
      for (var i = 0; i < defs.length; i++) {
        (function (val, label, pane, idx) {
          var b = el("button", "rvp-tab-btn" + (idx === 0 ? " active" : ""), label);
          btns[idx] = b;
          b.onclick = function () {
            for (var j = 0; j < btns.length; j++) btns[j].className = "rvp-tab-btn";
            b.className = "rvp-tab-btn active";
            content.innerHTML = "";
            content.appendChild(document.createTextNode(pane));
            fire("tab", "switch", val);
          };
          tabs.appendChild(b);
        })(defs[i][0], defs[i][1], defs[i][2], i);
      }
      wrap.appendChild(content);
      wrap.appendChild(tabs);
      g.appendChild(wrap);
    })();

    // --- scroll offset (transform, NOT scrollTop) --------------------------
    // transform/translateY still repaints the gradient/shadow DOM (Presto does
    // NOT composite here — proven on-unit), but one offset is cheaper to drive
    // than scrollTop and lets the rAF loop batch moves. ONE offset drives both
    // the finger-drag (injector, via dragStart/dragTo/dragStop) and the knob
    // (paint -> scrollToFocused). note: 1:1 drag, no flick momentum.
    var viewport = container.parentNode;   // the overflow:hidden scrollport (overlay minus header)
    var scrollY = 0;
    // Force a composited layer so the scroll transform COMPOSITES instead of
    // repainting each frame. translate3d (in setY) does it when 3D is supported;
    // opacity<1 is the 2D-safe fallback trigger Presto honors either way. Without
    // a layer, the engine re-rasterizes the gradient DOM every frame = the lag.
    try { container.style.opacity = "0.999"; } catch (e) {}
    function setY(v) {
      var s = container.style, t2 = "translateY(" + (-v) + "px)", t3 = "translate3d(0," + (-v) + "px,0)";
      s.OTransform = t2; s.OTransform = t3;        // -o- is the unit's prefix; 3d is a layer upgrade, silently dropped if unsupported
      s.webkitTransform = t2; s.webkitTransform = t3;
      s.transform = t2; s.transform = t3;
    }
    function applyScroll() {
      var vh = viewport ? viewport.clientHeight : (window.innerHeight || 480);
      var max = container.offsetHeight - vh; if (max < 0) max = 0;
      if (scrollY < 0) scrollY = 0; else if (scrollY > max) scrollY = max;
      setY(scrollY);
    }
    var raf = window.requestAnimationFrame || window.oRequestAnimationFrame ||
              window.msRequestAnimationFrame || function (cb) { return setTimeout(cb, 16); };
    // Input decoupled from paint. MEASURED on-unit: per-event painting backed
    // Opera's mousemove queue up to 7-15s, which then replayed the gesture
    // (burst<4ms~53%, stall>30ms~25%). So the pointer handler only stashes the
    // latest Y (dragTo); a single rAF loop, alive only while dragging, folds
    // every queued move since the last frame into ONE delta and paints once.
    // Draining 500 stale events just overwrites curY -> the list tracks the
    // live finger instead of walking the whole history. rvp-fast flattens the
    // shadows/glow so each frame rasterizes cheap, like stock's flat rows.
    var dragLive = false, curY = 0, frameY = 0, dragRAF = null;
    function dragStart(y) {
      dragLive = true; curY = y; frameY = y;
      if (container.className.indexOf("rvp-fast") < 0) container.className += " rvp-fast";
      if (!dragRAF) loop();
    }
    function dragTo(y) { curY = y; }   // cheap: stash only, no layout/paint
    function loop() {
      dragRAF = raf(function () {
        dragRAF = null;
        if (!dragLive) return;
        var dy = frameY - curY;        // finger up => positive => content scrolls up
        if (dy) { frameY = curY; scrollY += dy; applyScroll(); }
        loop();
      });
    }
    function dragStop() {
      dragLive = false;
      container.className = container.className.replace(/\s*rvp-fast\b/g, "");
    }
    function scrollToFocused(elc) {
      if (!elc || !viewport || !elc.getBoundingClientRect) return;
      var r = elc.getBoundingClientRect(), vr = viewport.getBoundingClientRect();
      var top = r.top - vr.top, bot = r.bottom - vr.top, vh = viewport.clientHeight;
      if (top < 8) scrollY += (top - 8);
      else if (bot > vh - 8) scrollY += (bot - (vh - 8));
      applyScroll();
    }

    // --- commander / knob navigation + input intercept ---------------------
    // On the head unit the primary input is the Commander knob, not touch.
    // Multicontroller keycodes (from jci-ui-vault 05-framework-modules):
    //   CW=77  CCW=78  Enter=13(select)  Backspace=8(GoBack)  arrows=37-40.
    // We keep a focus ring over every clickable control; rotate moves it,
    // press activates. The handler is installed in CAPTURE phase and swallows
    // the key so the HMI's own multicontroller does not also act on it.
    var FOCUS_SEL = ".rvp-back,.rvp-btn,.rvp-item,.rvp-radio-row,.rvp-acc-head,.rvp-tab-btn";
    var items = [], idx = -1;
    function collect() {
      var nl = container.querySelectorAll(FOCUS_SEL), a = [], i;
      for (i = 0; i < nl.length; i++) a.push(nl[i]);
      items = a;
    }
    function paint() {
      for (var i = 0; i < items.length; i++) {
        var c = items[i].className.replace(/\s*rvp-focus\b/g, "");
        items[i].className = (i === idx) ? c + " rvp-focus" : c;
      }
      if (idx >= 0 && items[idx]) scrollToFocused(items[idx]);
    }
    function move(d) {
      collect();
      if (!items.length) return;
      idx = (idx + d + items.length) % items.length;
      paint();
    }
    function activate() {
      if (idx < 0) { move(1); return; }
      var elc = items[idx];
      if (elc && typeof elc.onclick === "function") elc.onclick();
    }
    function onKey(e) {
      var k = e.keyCode, done = true;
      if (k === 77 || k === 39 || k === 40) move(1);         // CW / right / down -> next
      else if (k === 78 || k === 37 || k === 38) move(-1);   // CCW / left / up   -> prev
      else if (k === 13) activate();                          // select press
      else if (k === 8) { if (back.onclick) back.onclick(); } // GoBack
      else done = false;
      if (done) {
        if (e.preventDefault) e.preventDefault();
        if (e.stopPropagation) e.stopPropagation();
        if (e.stopImmediatePropagation) e.stopImmediatePropagation();
      }
    }
    document.addEventListener("keydown", onKey, true);

    collect();
    idx = 0;
    paint();

    // Controller for the host: call teardown() on close to release the knob.
    return {
      teardown: function () { try { document.removeEventListener("keydown", onKey, true); } catch (e) {} },
      focus: function (n) { collect(); idx = n % (items.length || 1); paint(); },
      next: function () { move(1); },
      prev: function () { move(-1); },
      activate: activate,
      dragStart: dragStart,
      dragTo: dragTo,
      dragStop: dragStop
    };
  }

  return { injectStyles: injectStyles, render: render };
})();

// CommonJS escape hatch (harmless in the browser / Presto).
if (typeof module !== "undefined" && module.exports) module.exports = RevisedTestApp;
