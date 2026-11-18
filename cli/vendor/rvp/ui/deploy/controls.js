/* Revised Store -- on-HMI store renderer (ES5, Opera/Presto ~2012).
 *
 * Built on the proven test-app renderer (controls.testapp-reference.js): same
 * render(host, send) contract, same class-based focus paint, fixed chrome on the
 * overlay. Content: burger + drawer, a screen stack per section (list -> package
 * page -> confirm / Pair), header slot (online, DEV, restart marker), modals.
 *
 * TARGET = 2012 Opera/Presto. Use only what Presto had:
 *   - no flex, no display:table/table-cell, no var(), no calc(), no clip-path;
 *   - no -webkit-* anything; prefix is -o-, then unprefixed (last valid wins);
 *   - gradients 3x: solid fallback -> -o-linear-gradient -> linear-gradient;
 *   - layout = inline-block + vertical-align + position:absolute;
 *   - ES5 only (var, function expressions, no template literals / Promise).
 *
 * Frame (owned by the host: the SDK overlay, or webapp/index.html on the desktop):
 *   overlay 800x480
 *     header 48px  (title, headerSlot, clock, back)  <- we add the burger on top
 *     scrollport top:48 bottom:0 overflow:hidden   <- `host.parentNode` = viewport
 *       host                                       <- render(host) builds content here
 *   Fixed chrome (burger, drawer, scrim, modal) goes on the overlay
 *   (= viewport.parentNode) so it never scrolls.
 *
 * API (global RevisedTestApp -- name kept, the injector calls it):
 *   RevisedTestApp.injectStyles()
 *   RevisedTestApp.render(host, send, opts) -> { teardown, onDaemon, key, keyUp, push, pop }
 *   opts.ctx (SDK): title via ctx.setTitle, header icons into ctx.headerSlot, keys via
 *   ctx.onKey (header back arrives as key 8), scrolling ONLY via ctx.scrollTo (the SDK
 *   owns the scroll + drag loop). Without ctx (desktop webapp): window key listeners
 *   (unless opts.bindKeys === false) and focus scrolling sets the viewport's scrollTop.
 *   send(evt) gets { control, action, value, ts } on every interaction.
 *   onDaemon(frame) takes the daemon's parsed JSON frames: catalog, plan, progress,
 *   result, error, asset, rebooting, pair, pair-done, usb; plus {t:"offline"} from the host when the link
 *   drops. Store actions go out as { control:"store", action, value, ts }.
 */
var RevisedTestApp = (function () {
  "use strict";

  var STYLE_ID = "rvp-store-styles";
  var RESTART_NOTE = "Menu changes appear after an HMI restart";   // design 2a: menus are built at HMI start
  var PINNED = "cmu-re-store";   // the store itself: listed in Apps, never removed here (updates: the update stick)
  // rating stars: constant SVG markup only (the HMI font may lack U+2605); no data goes into it
  var STAR_SVG = ' viewBox="0 0 24 24" width="14" height="14" xmlns="http://www.w3.org/2000/svg"><path d="M12 2.5l2.9 6.1 6.6.8-4.9 4.6 1.3 6.6L12 17.3l-5.9 3.3 1.3-6.6-4.9-4.6 6.6-.8z"/></svg>';
  var STAR_ON = '<svg class="rvp-star on"' + STAR_SVG, STAR_OFF = '<svg class="rvp-star off"' + STAR_SVG;
  // a row of `of` stars, the first round(v) filled
  function stars(v, of) {
    var e = el("span", "rvp-strow"), k = Math.round(Number(v)) || 0, h = "", i;
    for (i = 0; i < of; i++) h += i < k ? STAR_ON : STAR_OFF;
    e.innerHTML = h;
    return e;
  }

  // ---- content ----------------------------------------------------------------
  var ICONS = {
    // menu + back: owner-picked rounded-square glyphs (evenodd cuts the lines out of the square)
    menu: '<path fill-rule="evenodd" d="M16.19 2H7.81C4.17 2 2 4.17 2 7.81V16.18C2 19.83 4.17 22 7.81 22H16.18C19.82 22 21.99 19.83 21.99 16.19V7.81C22 4.17 19.83 2 16.19 2ZM17 17.25H7C6.59 17.25 6.25 16.91 6.25 16.5C6.25 16.09 6.59 15.75 7 15.75H17C17.41 15.75 17.75 16.09 17.75 16.5C17.75 16.91 17.41 17.25 17 17.25ZM17 12.75H7C6.59 12.75 6.25 12.41 6.25 12C6.25 11.59 6.59 11.25 7 11.25H17C17.41 11.25 17.75 11.59 17.75 12C17.75 12.41 17.41 12.75 17 12.75ZM17 8.25H7C6.59 8.25 6.25 7.91 6.25 7.5C6.25 7.09 6.59 6.75 7 6.75H17C17.41 6.75 17.75 7.09 17.75 7.5C17.75 7.91 17.41 8.25 17 8.25Z"/>',
    // dots: burger glyph while the drawer is open (owner pick; 256-unit source scaled into 24)
    dots: '<g transform="scale(0.09375)"><path d="M128,28A100,100,0,1,0,228,128,100.11332,100.11332,0,0,0,128,28Zm0,192a92,92,0,1,1,92-92A92.10416,92.10416,0,0,1,128,220Zm8-92a8,8,0,1,1-8-8A8.00917,8.00917,0,0,1,136,128Zm0,48a8,8,0,1,1-8-8A8.00917,8.00917,0,0,1,136,176Zm0-96a8,8,0,1,1-8-8A8.00917,8.00917,0,0,1,136,80Z"/></g>',
    back: '<path fill-rule="evenodd" d="M16.19 2H7.81C4.17 2 2 4.17 2 7.81V16.18C2 19.83 4.17 22 7.81 22H16.18C19.82 22 21.99 19.83 21.99 16.19V7.81C22 4.17 19.83 2 16.19 2ZM13.92 16.13H9C8.59 16.13 8.25 15.79 8.25 15.38C8.25 14.97 8.59 14.63 9 14.63H13.92C15.2 14.63 16.25 13.59 16.25 12.3C16.25 11.01 15.21 9.97 13.92 9.97H8.85L9.11 10.23C9.4 10.53 9.4 11 9.1 11.3C8.95 11.45 8.76 11.52 8.57 11.52C8.38 11.52 8.19 11.45 8.04 11.3L6.47 9.72C6.18 9.43 6.18 8.95 6.47 8.66L8.04 7.09C8.33 6.8 8.81 6.8 9.1 7.09C9.39 7.38 9.39 7.86 9.1 8.15L8.77 8.48H13.92C16.03 8.48 17.75 10.2 17.75 12.31C17.75 14.42 16.03 16.13 13.92 16.13Z"/>',
    home: '<path d="M12 3L2 11h3v9h5v-6h4v6h5v-9h3z"/>',
    apps: '<path d="M3 3h8v8H3zM13 3h8v8h-8zM3 13h8v8H3zM13 13h8v8h-8z"/>',
    mods: '<path d="M19.4 13a7.5 7.5 0 000-2l2.1-1.6-2-3.5-2.5 1a7 7 0 00-1.7-1L15 3h-4l-.4 2.6a7 7 0 00-1.7 1l-2.5-1-2 3.5L6.6 11a7.5 7.5 0 000 2l-2.1 1.6 2 3.5 2.5-1a7 7 0 001.7 1L11 21h4l.4-2.6a7 7 0 001.7-1l2.5 1 2-3.5zM13 15.5a3.5 3.5 0 110-7 3.5 3.5 0 010 7z"/>',
    skin: '<path d="M12 3a9 9 0 000 18c1 0 1.5-.7 1.5-1.5 0-.4-.2-.8-.4-1-.3-.3-.4-.6-.4-1 0-.8.7-1.5 1.5-1.5H16a5 5 0 005-5c0-4.4-4-8-9-8zM6.5 12a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm3-4a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm5 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm3 4a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>',
    user: '<path d="M12 12a4.5 4.5 0 100-9 4.5 4.5 0 000 9zm0 2c-4 0-8 2-8 5v2h16v-2c0-3-4-5-8-5z"/>',
    down: '<path d="M7 10l5 5 5-5z"/>',
    up: '<path d="M7 14l5-5 5 5z"/>',
    x: '<path d="M19 6.4L17.6 5 12 10.6 6.4 5 5 6.4 10.6 12 5 17.6 6.4 19 12 13.4 17.6 19 19 17.6 13.4 12z"/>',
    // card state icons (owner-picked svgrepo glyphs): installed / update / unsigned; stroke colour from CSS
    info: '<path fill="none" stroke-width="1.5" stroke-linecap="round" d="M12 17V11"/><circle cx="12" cy="8" r="1"/>' +
      '<path fill="none" stroke-width="1.5" d="M2 12C2 7.29 2 4.93 3.46 3.46 4.93 2 7.29 2 12 2c4.71 0 7.07 0 8.54 1.46C22 4.93 22 7.29 22 12c0 4.71 0 7.07-1.46 8.54C19.07 22 16.71 22 12 22c-4.71 0-7.07 0-8.54-1.46C2 19.07 2 16.71 2 12z"/>',
    upd: '<g transform="scale(1.2632) translate(1.5 0)"><path d="M5.857 3.882v3.341a1.03 1.03 0 0 1-2.058 0v-.97a5.401 5.401 0 0 0-1.032 2.27 1.03 1.03 0 1 1-2.02-.395A7.462 7.462 0 0 1 2.235 4.91h-.748a1.03 1.03 0 1 1 0-2.058h3.34a1.03 1.03 0 0 1 1.03 1.03zm-3.25 9.237a1.028 1.028 0 0 1-1.358-.523 7.497 7.497 0 0 1-.37-1.036 1.03 1.03 0 1 1 1.983-.55 5.474 5.474 0 0 0 .269.751 1.029 1.029 0 0 1-.524 1.358zm2.905 2.439a1.028 1.028 0 0 1-1.42.322 7.522 7.522 0 0 1-.885-.652 1.03 1.03 0 0 1 1.34-1.563 5.435 5.435 0 0 0 .643.473 1.03 1.03 0 0 1 .322 1.42zm3.68.438a1.03 1.03 0 0 1-1.014 1.044h-.106a7.488 7.488 0 0 1-.811-.044 1.03 1.03 0 0 1 .224-2.046 5.41 5.41 0 0 0 .664.031h.014a1.03 1.03 0 0 1 1.03 1.015zm.034-12.847a1.03 1.03 0 0 1-1.029 1.01h-.033a1.03 1.03 0 0 1 .017-2.06h.017l.019.001a1.03 1.03 0 0 1 1.009 1.05zm3.236 11.25a1.029 1.029 0 0 1-.3 1.425 7.477 7.477 0 0 1-.797.453 1.03 1.03 0 1 1-.905-1.849 5.479 5.479 0 0 0 .578-.328 1.03 1.03 0 0 1 1.424.3zM10.475 3.504a1.029 1.029 0 0 1 1.41-.359l.018.011a1.03 1.03 0 1 1-1.06 1.764l-.01-.006a1.029 1.029 0 0 1-.358-1.41zm4.26 9.445a7.5 7.5 0 0 1-.315.56 1.03 1.03 0 1 1-1.749-1.086 5.01 5.01 0 0 0 .228-.405 1.03 1.03 0 1 1 1.836.93zm-1.959-6.052a1.03 1.03 0 0 1 1.79-1.016l.008.013a1.03 1.03 0 1 1-1.79 1.017zm2.764 2.487a9.327 9.327 0 0 1 0 .366 1.03 1.03 0 0 1-1.029 1.005h-.025A1.03 1.03 0 0 1 13.482 9.7a4.625 4.625 0 0 0 0-.266 1.03 1.03 0 0 1 1.003-1.055h.026a1.03 1.03 0 0 1 1.029 1.004z"/></g>',
    warn: '<g fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><path d="M12 9v4M12 17h.01"/></g>',
    pkg:  '<path d="M12 2L3 7v10l9 5 9-5V7zm0 2.3L18.6 8 12 11.7 5.4 8zM5 9.7l6 3.4v6.6l-6-3.4zm8 10v-6.6l6-3.4v6.6z"/>'
  };
  function svg(name) {
    return '<svg class="rvp-i" viewBox="0 0 24 24" width="24" height="24" xmlns="http://www.w3.org/2000/svg">' +
      (ICONS[name] || "") + "</svg>";
  }

  var MENU = [
    { key: "Home", ic: "home" },
    { key: "Apps", ic: "apps" },
    { key: "SysMods", ic: "mods" },
    { key: "Skins", ic: "skin" },
    { key: "Developer", ic: "mods", dev: true },   // only while catalog.devmode
    { key: "Account", ic: "user" }
  ];
  var SSH_PKG = "cmu-re-dev-ssh";
  var TW_OFF = "Tripwire is OFF: if this package breaks boot, only a reflash recovers the unit.";
  // catalog `category` -> section; note: unknown/empty (installed, gone from the catalog) lands in Apps
  var SECTION_OF = { "Apps": "Apps", "System mods": "SysMods", "Skins": "Skins" };
  var PERM_WORDS = { audio: "Audio", input: "Input", "system-dbus": "System bus" };
  var STAGE = { download: "Downloading", verify: "Verifying", apply: "Installing", revert: "Removing" };

  // ---- styles -------------------------------------------------------------------
  var FONT = "font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;";
  var RED = "background:#B50E13;background:-o-linear-gradient(top,#E11218,#900B0F);background:linear-gradient(to bottom,#E11218,#900B0F);";
  var CSS =
    ".rvp-testapp{" + FONT + "background:#000;color:#E0E0E0;}" +
    ".rvp-testapp *,.rvp-chrome,.rvp-chrome *{box-sizing:border-box;margin:0;padding:0;}" +
    ".rvp-chrome{" + FONT + "color:#E0E0E0;}" +
    ".rvp-i{display:block;width:24px;height:24px;fill:#FFFFFF;}" +
    /* header buttons, sit on top of the host header (z-index 2) */
    ".rvp-hbtn{position:absolute;top:0;width:52px;height:48px;z-index:3;cursor:pointer;}" +
    ".rvp-hbtn .rvp-i{position:absolute;left:12px;top:10px;width:28px;height:28px;}" +
    ".rvp-burger{left:0;}" +
    ".rvp-hback{right:0;}" +
    /* header slot: online dot, DEV, restart marker (inline, the host positions the slot) */
    ".rvp-slot{position:absolute;right:60px;top:0;height:48px;line-height:48px;z-index:3;}" +
    ".rvp-online{display:inline-block;vertical-align:middle;width:12px;height:12px;border-radius:6px;background:#444444;margin:0 10px;}" +
    ".rvp-online.on{background:#2ECC40;}" +
    ".rvp-dev,.rvp-pend{display:inline-block;vertical-align:middle;height:24px;line-height:22px;padding:0 8px;margin:0 6px;border-radius:4px;font-size:13px;font-weight:600;}" +
    ".rvp-dev{color:#FFD23F;border:1px solid #FFD23F;}" +
    ".rvp-pend{color:#FFFFFF;border:1px solid #F03A3F;cursor:pointer;" + RED + "}" +
    /* list + cards: text in flow, icon + badge absolute (no table) */
    ".rvp-list{padding:12px 16px 16px;}" +
    ".rvp-card{position:relative;min-height:72px;margin:0 0 10px;padding:14px 130px 14px 72px;background:#141414;border:1px solid #1C1C1C;border-radius:6px;cursor:pointer;}" +
    ".rvp-ico{position:absolute;left:14px;top:50%;margin-top:-22px;width:44px;height:44px;border-radius:10px;background:#1E1E1E;border:1px solid #2A2A2A;overflow:hidden;}" +
    ".rvp-ico .rvp-i{position:absolute;left:9px;top:9px;fill:#DCDCDC;}" +
    ".rvp-img{display:block;width:100%;height:100%;}" +
    ".rvp-grey{background:#3A3A3A;}" +
    ".rvp-nm{font-size:18px;color:#FFFFFF;line-height:24px;}" +
    ".rvp-sub{font-size:13px;color:#8A8A8A;margin-top:4px;white-space:nowrap;overflow:hidden;-o-text-overflow:ellipsis;text-overflow:ellipsis;}" +
    ".rvp-sub.rvp-err{color:#E05050;white-space:normal;}" +
    ".rvp-badge{position:absolute;right:14px;top:14px;height:22px;line-height:20px;padding:0 8px;border-radius:4px;font-size:12px;font-weight:600;color:#FFFFFF;border:1px solid #F03A3F;" + RED + "}" +
    ".rvp-bi{position:absolute;right:16px;top:50%;margin-top:-16px;}" +
    ".rvp-bi .rvp-i{width:32px;height:32px;}" +
    ".rvp-bi-info .rvp-i{fill:#9A9A9A;stroke:#9A9A9A;}" +
    ".rvp-bi-upd .rvp-i{fill:#FF2A30;}" +
    ".rvp-bi-warn .rvp-i{stroke:#FFC400;}" +
    ".rvp-chips{margin:0 0 10px;}" +
    ".rvp-chips .rvp-btn{height:32px;line-height:30px;padding:0 16px;font-size:14px;margin-right:8px;}" +
    ".rvp-stars{position:absolute;right:14px;bottom:12px;font-size:13px;color:#CFCFCF;}" +
    ".rvp-strow{display:inline-block;vertical-align:middle;height:14px;line-height:14px;margin:0 4px;}" +
    ".rvp-star{display:inline-block;vertical-align:top;width:14px;height:14px;margin-right:1px;}" +
    ".rvp-star.on{fill:#E11218;}" +
    ".rvp-star.off{fill:none;stroke:#5A5A5A;stroke-width:2;}" +
    /* progress bar under the subtitle; fill width = stage % */
    ".rvp-bar{position:absolute;left:72px;right:130px;bottom:8px;height:4px;background:#2A2A2A;border-radius:2px;overflow:hidden;}" +
    ".rvp-fill{height:4px;width:0;background:#F03A3F;}" +
    ".rvp-pgbox{position:relative;margin:14px 0;height:40px;}" +
    ".rvp-pgbox .rvp-bar{left:0;right:0;bottom:4px;}" +
    ".rvp-msg{margin:0 0 10px;padding:10px 14px;font-size:14px;line-height:20px;color:#E05050;background:#1A0708;border:1px solid #3A0C0E;border-radius:6px;}" +
    ".rvp-empty{padding:48px 16px;text-align:center;color:#8A8A8A;font-size:15px;}" +
    /* pane (home / account / confirm / pair) */
    ".rvp-pane{padding:32px 24px;}" +
    ".rvp-h1{font-size:26px;color:#FFFFFF;margin-bottom:10px;}" +
    ".rvp-h2{font-size:18px;color:#FFFFFF;margin:8px 0;}" +
    ".rvp-p{font-size:15px;color:#8A8A8A;line-height:22px;margin-bottom:6px;}" +
    ".rvp-li{font-size:17px;color:#FFFFFF;line-height:28px;}" +
    ".rvp-news{margin:0 0 10px;padding:10px 14px;background:#141414;border:1px solid #1C1C1C;border-radius:6px;}" +
    ".rvp-banner{margin:0 0 14px;padding:14px 16px;background:#1A0708;border:1px solid #3A0C0E;border-left:4px solid #E11218;border-radius:6px;cursor:pointer;}" +
    /* Pair screen (design/pair-qr-mockup.html): screen y minus the 48px header */
    ".rvp-pair{position:relative;width:800px;height:432px;background:#0C0C0C;overflow:hidden;}" +
    ".rvp-qr{position:absolute;left:0;top:0;width:800px;height:432px;}" +
    ".rvp-pt{position:absolute;left:440px;right:16px;white-space:nowrap;overflow:hidden;}" +
    ".rvp-pt1{top:64px;line-height:36px;font-size:30px;font-weight:bold;color:#FFFFFF;}" +
    ".rvp-pt2{top:112px;line-height:24px;font-size:18px;color:#8A8A8A;}" +
    ".rvp-pt3{top:138px;line-height:24px;font-size:18px;color:#8A8A8A;}" +
    ".rvp-purl{top:172px;line-height:24px;font-size:18px;color:#E0E0E0;}" +
    ".rvp-pbox{position:absolute;left:440px;top:214px;width:300px;height:64px;background:#2A2A2A;border-radius:10px;}" +
    ".rvp-plab{position:absolute;left:18px;top:11px;line-height:18px;font-size:14px;color:#8A8A8A;}" +
    ".rvp-pcode{position:absolute;left:18px;top:30px;line-height:34px;font-size:30px;font-weight:bold;font-family:monospace;color:#FFFFFF;}" +
    ".rvp-pexp{top:302px;line-height:20px;font-size:16px;color:#8A8A8A;}" +
    ".rvp-phint{top:363px;line-height:18px;font-size:14px;color:#5A5A5A;}" +
    /* package page */
    ".rvp-page{padding:16px 24px 24px;}" +
    ".rvp-btns{margin:14px 0;}" +
    ".rvp-btn{display:inline-block;vertical-align:middle;margin-right:12px;height:42px;line-height:40px;padding:0 24px;border-radius:5px;font-size:15px;font-weight:600;color:#FFFFFF;border:1px solid #F03A3F;cursor:pointer;" + RED + "}" +
    ".rvp-btn.ghost{background:#1C1C1C;border:1px solid #444444;color:#CFCFCF;}" +
    ".rvp-cfg{padding:16px 20px;border-bottom:1px solid #2A2C33;font-size:16px;line-height:22px;color:#FFFFFF;cursor:pointer;}" +
    ".rvp-cfgval{float:right;color:#8A94A6;}" +
    ".rvp-sw{position:relative;width:48px;height:26px;margin:-2px 0;border:1px solid #8A94A6;border-radius:13px;}" +   // border-box: 46x24 inside
    ".rvp-swk{position:absolute;top:3px;left:3px;width:18px;height:18px;border-radius:9px;background:#8A94A6;}" +
    ".rvp-sw.on{border-color:#F03A3F;background:#E11218;}" +
    ".rvp-sw.on .rvp-swk{left:25px;background:#FFFFFF;}" +
    ".rvp-rng{position:relative;height:6px;margin:16px 10px 6px;border-radius:3px;background:#3A3D46;}" +
    ".rvp-rngf{height:6px;border-radius:3px;background:#E11218;}" +
    ".rvp-rngk{position:absolute;top:-6px;width:18px;height:18px;margin-left:-9px;border-radius:9px;background:#FFFFFF;}" +
    ".rvp-cfg.rvp-edit .rvp-rngk{top:-8px;width:22px;height:22px;margin-left:-11px;border-radius:11px;}" +
    ".rvp-num{margin:-6px 0;}" +
    ".rvp-numb,.rvp-numv{display:inline-block;height:32px;line-height:32px;border:1px solid #3A3D46;text-align:center;color:#FFFFFF;}" +
    ".rvp-numb{width:36px;background:#1A1C22;font-weight:600;}" +
    ".rvp-numb:first-child{border-radius:5px 0 0 5px;}" +
    ".rvp-numb:last-child{border-radius:0 5px 5px 0;}" +
    ".rvp-numv{width:48px;border-left:0;border-right:0;background:#000000;}" +
    ".rvp-cfg.rvp-edit .rvp-numv{border-color:#E11218;}" +
    ".rvp-off{margin:0 20px 16px;padding:4px 16px 12px;border:1px solid #3A3D46;border-radius:5px;}" +
    ".rvp-offl{margin:4px 0 0 22px;padding:0;color:#FFFFFF;font-size:15px;line-height:24px;}" +
    ".rvp-cfg.dis{opacity:0.4;cursor:default;}" +
    ".rvp-cfg.rvp-edit{border-left:4px solid #E11218;padding-left:16px;}" +
    ".rvp-cfg.rvp-edit .rvp-cfgval{color:#FFFFFF;font-weight:600;}" +
    ".rvp-btn.dis{background:#1C1C1C;border:1px solid #2A2A2A;color:#5A5A5A;cursor:default;}" +
    ".rvp-dtrack{position:relative;height:46px;margin:14px 0;border:1px solid #444444;border-radius:5px;background:#1C1C1C;}" +
    ".rvp-dlabel{text-align:center;line-height:46px;font-size:15px;color:#CFCFCF;}" +
    ".rvp-dhandle{position:absolute;left:0;top:0;width:70px;height:46px;z-index:1;border-radius:5px;" + RED + "}" +
    ".rvp-line{font-size:15px;color:#CFCFCF;line-height:24px;margin:6px 0;}" +
    ".rvp-shots{position:relative;margin:12px 0;white-space:nowrap;}" +
    ".rvp-shot{display:inline-block;vertical-align:top;width:232px;height:130px;margin-right:12px;background:#1E1E1E;border:1px solid #2A2A2A;overflow:hidden;cursor:pointer;}" +
    ".rvp-shot.rvp-grey{background:#3A3A3A;}" +
    ".rvp-cnt{position:absolute;right:8px;bottom:20px;font-size:13px;color:#FFFFFF;}" +
    /* accordion: ui-kit mzd-accordion (framed group, gradient headers, red bar on the open one) */
    ".rvp-accs{margin:16px 0;border:1px solid #2A2C33;border-radius:4px;overflow:hidden;}" +
    ".rvp-acci{border-bottom:1px solid #2A2C33;}" +
    ".rvp-acci:last-child{border-bottom:0;}" +
    ".rvp-acc{position:relative;padding:16px 52px 16px 20px;font-size:16px;font-weight:500;line-height:22px;color:#FFFFFF;cursor:pointer;" +
      "background:#232429;background:-o-linear-gradient(top,#2A2C33,#1C1D21);background:linear-gradient(to bottom,#2A2C33,#1C1D21);}" +
    ".rvp-acci.open .rvp-acc{border-left:4px solid #E11218;padding-left:16px;}" +
    ".rvp-acc .rvp-i{position:absolute;right:16px;top:15px;}" +
    ".rvp-accb{padding:20px;background:#000000;color:#8A94A6;line-height:24px;}" +
    ".rvp-chg{font-size:14px;color:#8A8A8A;line-height:22px;}" +
    /* package page, store-listing style: title + author + stats left, big icon right */
    ".rvp-phead{position:relative;min-height:112px;padding:0 130px 0 0;}" +
    ".rvp-phead .rvp-ico{left:auto;right:0;top:0;margin-top:0;width:112px;height:112px;border-radius:24px;}" +
    ".rvp-phead .rvp-ico .rvp-i{left:28px;top:28px;width:56px;height:56px;}" +
    ".rvp-ptitle{font-size:30px;line-height:38px;color:#FFFFFF;}" +
    ".rvp-pauth{font-size:15px;line-height:22px;color:#F03A3F;margin-top:4px;}" +
    ".rvp-stats{margin-top:14px;white-space:nowrap;}" +
    ".rvp-stat{display:inline-block;vertical-align:top;padding:0 18px;border-left:1px solid #2A2A2A;text-align:center;}" +
    ".rvp-stat.first{padding-left:0;border-left:0;text-align:left;}" +
    ".rvp-stv{font-size:16px;line-height:22px;color:#FFFFFF;}" +
    ".rvp-stv .rvp-star{margin-left:3px;vertical-align:middle;}" +
    ".rvp-rbtn{height:36px;line-height:34px;padding:0 14px;margin:4px 0 0 6px;vertical-align:top;font-size:14px;}" +
    ".rvp-rbtn .rvp-strow{margin:0;}" +
    ".rvp-stl{font-size:13px;line-height:18px;color:#8A8A8A;}" +
    /* drawer + scrim (modal, fixed) */
    ".rvp-scrim,.rvp-mscrim{position:absolute;left:0;right:0;top:48px;bottom:0;background:#000000;opacity:0.5;z-index:4;display:none;}" +
    ".rvp-draw{position:absolute;left:0;top:48px;bottom:0;width:280px;background:#0D0D0D;border-right:1px solid #2A2A2A;z-index:5;display:none;}" +
    ".rvp-mscrim{z-index:6;opacity:0.7;}" +
    ".rvp-modal{position:absolute;left:140px;right:140px;top:100px;padding:24px;background:#141414;border:1px solid #3A0C0E;border-radius:8px;z-index:7;display:none;}" +
    ".rvp-on{display:block;}" +
    ".rvp-mshot{left:40px;right:40px;top:58px;padding:0;background:#000000;border:0;border-radius:0;}" +
    ".rvp-mshot .rvp-shots,.rvp-mshot .rvp-shot{display:block;width:720px;height:405px;margin:0 auto;}" +
    ".rvp-mshot .rvp-shots.rvp-focus{outline:0;box-shadow:none;}" +
    ".rvp-mshot .rvp-cnt{bottom:8px;}" +
    ".rvp-mx{position:absolute;right:8px;top:8px;width:44px;height:44px;border-radius:22px;background:#000000;opacity:0.7;cursor:pointer;}" +
    ".rvp-mx .rvp-i{margin:10px;}" +
    ".rvp-mrate{left:210px;right:210px;top:150px;padding:24px 0;text-align:center;}" +
    ".rvp-pstar{display:inline-block;width:60px;height:60px;margin:0 6px;cursor:pointer;}" +
    ".rvp-pstar .rvp-star{width:60px;height:60px;margin:0;}" +
    ".rvp-pstar.rvp-focus{outline:0;box-shadow:none;}" +
    ".rvp-ditem{position:relative;height:56px;line-height:56px;padding:0 18px 0 54px;font-size:18px;color:#E8E8E8;border-bottom:1px solid #141414;border-left:4px solid #0D0D0D;cursor:pointer;}" +
    ".rvp-ditem .rvp-i{position:absolute;left:18px;top:16px;fill:#9A9A9A;}" +
    ".rvp-ditem.act{background:#1A0708;border-left:4px solid #E11218;}" +
    ".rvp-ditem.act .rvp-i{fill:#FFFFFF;}" +
    /* knob focus ring: the only visible focus */
    ".rvp-focus{outline:1px solid #FF2A30;outline-offset:-1px;box-shadow:0 0 20px #B50E13;}" +
    /* accordion header: no red frame, a lighter header gradient marks the knob focus */
    ".rvp-acc.rvp-focus{outline:0;box-shadow:none;background:#44464F;background:-o-linear-gradient(top,#50525C,#383A42);background:linear-gradient(to bottom,#50525C,#383A42);}" +
    /* while the SDK drags the scrollport: no shadows -> cheap per-frame raster */
    ".rvp-dragging *{box-shadow:none !important;text-shadow:none !important;}";

  function injectStyles() {
    if (document.getElementById(STYLE_ID)) return;
    var s = document.createElement("style");
    s.id = STYLE_ID;
    s.type = "text/css";
    s.appendChild(document.createTextNode(CSS));
    (document.head || document.getElementsByTagName("head")[0] || document.body).appendChild(s);
  }

  // all catalog text goes in as a text node, never as markup
  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.appendChild(document.createTextNode(String(text)));
    return e;
  }

  // slide-to-confirm: drag the handle to >= 90% of the track, release -> onAgree once; else snaps back.
  // No rvp-act class: the knob never focuses it. Mouse events only (touch arrives as mouse on the HMI).
  function dragGate(label, onAgree) {
    var track = el("div", "rvp-dtrack"), handle = track.appendChild(el("div", "rvp-dhandle")), x0 = 0, l0 = 0, done = false;
    var pend = 0, timer = null;
    track.appendChild(el("div", "rvp-dlabel", label));
    function range() { return track.offsetWidth - handle.offsetWidth; }
    // Presto does not repaint per mousemove while the button is held (same paint-bound queue
    // as the overlay scroll): move only stores the X, one 40 ms timer paints the handle.
    function paintH() { handle.style.left = pend + "px"; }
    function stop() {
      document.removeEventListener("mousemove", move, false);
      document.removeEventListener("mouseup", up, false);
      if (timer) { clearInterval(timer); timer = null; }
    }
    // the modal was closed / re-rendered mid-drag: drop the drag, never agree for a gone track
    function move(e) {
      if (!track.parentNode) { stop(); return; }
      pend = Math.max(0, Math.min(range(), l0 + e.clientX - x0));
    }
    function up() {
      var l = pend;
      stop();
      if (!track.parentNode) return;
      if (!done && range() > 0 && l >= 0.9 * range()) { done = true; paintH(); onAgree(); }
      else { pend = 0; paintH(); }
    }
    handle.onmousedown = function (e) {
      x0 = e.clientX; l0 = pend;
      document.addEventListener("mousemove", move, false);
      document.addEventListener("mouseup", up, false);
      if (!timer) timer = setInterval(paintH, 40);
      if (e.preventDefault) e.preventDefault();
    };
    return track;
  }

  // ---- Pair badge: QR style "A3 tachometer" (owner 2026-11-14, design/pair-qr-mockup.html) ----
  // rows = daemon `pair.qr` ("1" = dark). Canvas = the 800x432 content under the 48px header,
  // so screen y 269 -> 221. Runs the 1.4 s startup animation on setTimeout (no rAF in Presto);
  // returns stop(). Rounded squares: arc() corners + lineTo, NEVER arcTo (Presto 12 garbage).
  var PAIR_BG = "#0C0C0C";
  function drawQr(cv, rows) {
    var c = cv.getContext ? cv.getContext("2d") : null;
    if (!c || !rows.length) return function () {};
    var P = Math.PI, n = rows.length, CX = 220, CY = 269 - 48, R = 175, M = Math.ceil(n * 1.42) + 4;
    if ((M - n) % 2) M++;
    var s = 2 * R / M, off = (M - n) / 2, ox = CX - R, oy = CY - R;
    var L = Math.floor(n / 2) - 4, H = Math.floor(n / 2) + 4, seed = 7, cells = [], A0 = P * 0.75, SW = P * 1.5;
    var EYES = [[0, 0], [0, n - 7], [n - 7, 0]], r, cc, x, y, d, qr, qc, g, timer = null, t0;
    function rnd() { seed = (seed * 16807) % 2147483647; return seed / 2147483647; }
    function eye(r, cc) { return (r < 7 && cc < 7) || (r < 7 && cc >= n - 7) || (r >= n - 7 && cc < 7); }
    function rr(x, y, w, h, r) {
      c.beginPath(); c.moveTo(x + r, y);
      c.lineTo(x + w - r, y); c.arc(x + w - r, y + r, r, -P / 2, 0, false);
      c.lineTo(x + w, y + h - r); c.arc(x + w - r, y + h - r, r, 0, P / 2, false);
      c.lineTo(x + r, y + h); c.arc(x + r, y + h - r, r, P / 2, P, false);
      c.lineTo(x, y + r); c.arc(x + r, y + r, r, P, P * 1.5, false);
      c.closePath(); c.fill();
    }
    // body dots (dark modules minus eyes and the 9x9 logo box) + decorative dots outside the code + 2-module gap
    for (r = 0; r < M; r++) for (cc = 0; cc < M; cc++) {
      x = ox + cc * s + s / 2; y = oy + r * s + s / 2; d = Math.sqrt((x - CX) * (x - CX) + (y - CY) * (y - CY));
      qr = r - off; qc = cc - off;
      if (qr >= -2 && qc >= -2 && qr < n + 2 && qc < n + 2) {
        if (qr < 0 || qc < 0 || qr >= n || qc >= n) continue;
        if (rows[qr].charAt(qc) !== "1" || eye(qr, qc) || (qr >= L && qr <= H && qc >= L && qc <= H)) continue;
        cells.push([x, y, d, 1]);
      } else if (d < R - s * 0.6 && rnd() < 0.45) cells.push([x, y, d, 0]);
    }
    g = c.createLinearGradient(ox + off * s, oy + off * s, ox + (off + n) * s, oy + (off + n) * s);
    g.addColorStop(0, "#FF2A2A"); g.addColorStop(1, "#9E0000");
    function frame(p) {
      var reach = p * (R * 1.05), i, e, t, an, maj, r1, r2, bx, by, bw, sw, na;
      c.fillStyle = PAIR_BG; c.fillRect(0, 0, 800, 432);
      for (i = 0; i < cells.length; i++) {   // dots appear from the centre outwards
        e = cells[i];
        if (e[2] > reach) continue;
        c.fillStyle = e[3] ? g : "#3A0C0E"; c.beginPath(); c.arc(e[0], e[1], s * 0.42, 0, 7); c.fill();
      }
      if (p > 0.55) {   // eyes + logo fade in
        c.globalAlpha = Math.min(1, (p - 0.55) / 0.3);
        for (i = 0; i < EYES.length; i++) {
          x = ox + (off + EYES[i][1]) * s; y = oy + (off + EYES[i][0]) * s;
          c.fillStyle = g; rr(x, y, 7 * s, 7 * s, 3.5 * s);
          c.fillStyle = PAIR_BG; rr(x + s, y + s, 5 * s, 5 * s, 2.5 * s);
          c.fillStyle = g; rr(x + 2 * s, y + 2 * s, 3 * s, 3 * s, 1.5 * s);
        }
        // note: logo-minimal.svg arc over a bold RE, placeholder until a square badge exists
        bx = ox + (off + L + 0.5) * s; by = oy + (off + L + 0.5) * s; bw = (H - L) * s;
        c.strokeStyle = "#E11218"; c.lineWidth = bw * 0.06;
        c.beginPath(); c.arc(bx + bw / 2, by + bw * 0.62, bw * 0.36, P * 1.08, P * 1.92); c.stroke();
        c.fillStyle = "#FFFFFF"; c.font = "bold " + Math.round(bw * 0.38) + "px sans-serif";
        c.textAlign = "center"; c.textBaseline = "middle"; c.fillText("RE", bx + bw / 2, by + bw * 0.62);
        c.globalAlpha = 1; c.textAlign = "left";
      }
      c.lineCap = "round";
      for (t = 0; t <= 40; t++) {   // tachometer ring: 40 ticks over 270 deg, 32-40 = redline
        an = A0 + SW * t / 40; maj = t % 5 === 0; r1 = R + (maj ? 10 : 14); r2 = R + 24;
        c.strokeStyle = t >= 32 ? "#E11218" : (maj ? "#CFCFCF" : "#5A5A5A"); c.lineWidth = maj ? 3 : 1.5;
        c.beginPath(); c.moveTo(CX + Math.cos(an) * r1, CY + Math.sin(an) * r1); c.lineTo(CX + Math.cos(an) * r2, CY + Math.sin(an) * r2); c.stroke();
      }
      c.strokeStyle = "#E11218"; c.lineWidth = 4;
      c.beginPath(); c.arc(CX, CY, R + 28, A0 + SW * 0.8, A0 + SW); c.stroke();
      sw = p < 0.5 ? p / 0.5 : 1 - (p - 0.5) / 0.5; na = A0 + SW * Math.max(0, sw);   // needle 0 -> redline -> 0
      c.strokeStyle = "#FF2A2A"; c.lineWidth = 3;
      c.beginPath(); c.moveTo(CX + Math.cos(na) * (R + 4), CY + Math.sin(na) * (R + 4)); c.lineTo(CX + Math.cos(na) * (R + 30), CY + Math.sin(na) * (R + 30)); c.stroke();
    }
    function run() {
      var p = Math.min(1, (Date.now() - t0) / 1400);
      frame(p);
      timer = p < 1 ? setTimeout(run, 33) : null;
    }
    t0 = Date.now();
    run();
    return function () { if (timer) clearTimeout(timer); timer = null; };
  }

  // ---- render -------------------------------------------------------------------
  function render(container, send, opts) {
    opts = opts || {};
    var ctx = opts.ctx || null;
    var bindKeys = !ctx && opts.bindKeys !== false;   // SDK: keys come through ctx.onKey
    var emit = typeof send === "function" ? send : function () {};
    function fire(control, action, value) {
      emit({ control: control, action: action, value: value, ts: +new Date() });
    }
    function store(action, value) { fire("store", action, value); }

    container.className = (container.className ? container.className + " " : "") + "rvp-testapp";
    container.innerHTML = "";
    try { container.style.opacity = "0.999"; } catch (e) {}
    var viewport = container.parentNode;                  // scrollport (clips the host)
    var frame = viewport ? viewport.parentNode : null;    // 800x480 overlay: fixed chrome goes here
    var chrome = [];
    function addChrome(node) { if (frame) { frame.appendChild(node); chrome.push(node); } return node; }

    var section = "Apps", drawerOpen = false, filt = "";
    var stack = [{ kind: "root", idx: -1 }];   // screen stack of the current section; [0] = section root
    var modal = null;        // { kind:"restart", busy, err } | { kind:"rate", name }
    // store state fed by onDaemon
    var cat = null;          // last catalog frame; null = no daemon -> No catalog yet
    var errs = {};           // name -> error text (red) until a later catalog
    var keepErr = false;     // a failed result is followed at once by a catalog: let that one keep errs
    var prog = {};           // name -> last progress frame
    var subs = {};           // name -> progress text node of the rendered card / page (in place)
    var asked = null;        // { name, op } we sent `plan`/`plan-update` for (frames are broadcast)
    var topMsg = "";         // daemon error / warning / result line on top of the screen
    var pending = [];        // titles waiting for a head unit restart (catalog.restartPending)
    var sawCat = false;      // first catalog of this overlay: a pending restart opens the modal at once
    var assets = {}, req = {}, assetNode = {};   // "name/kind/i" -> asset frame | asked | nodes waiting for it
    var pairT = {};          // Pair screen timers: { stop (badge animation), cd (countdown) }

    // --- fixed chrome ---
    var burger = addChrome(el("div", "rvp-chrome rvp-hbtn rvp-burger"));
    burger.innerHTML = svg("menu");
    var back = null;
    if (!ctx) {   // the SDK header has its own back (arrives as key 8)
      back = addChrome(el("div", "rvp-chrome rvp-hbtn rvp-hback"));
      back.innerHTML = svg("back");
      back.onclick = function () { keyDown(8); };
    }
    var slot = ctx ? ctx.headerSlot : addChrome(el("div", "rvp-chrome rvp-slot"));
    var scrim = addChrome(el("div", "rvp-chrome rvp-scrim"));
    var draw = addChrome(el("div", "rvp-chrome rvp-draw"));
    var mscrim = addChrome(el("div", "rvp-chrome rvp-mscrim"));
    var modalEl = addChrome(el("div", "rvp-chrome rvp-modal"));
    var titleEl = ctx ? null : document.getElementById("rvp-inj-title");
    function setTitle(t) { if (ctx) ctx.setTitle(t); else if (titleEl) titleEl.textContent = t; }

    var ditems = [], drawDev = null;   // drawDev: devmode the drawer was built for
    function buildDrawer(dev) {
      drawDev = dev; ditems = []; draw.innerHTML = "";
      for (var m = 0; m < MENU.length; m++) {
        if (MENU[m].dev && !dev) continue;
        (function (mi) {
          var it = el("div", "rvp-ditem");
          it.innerHTML = svg(mi.ic);
          it.appendChild(document.createTextNode(mi.key));
          it.onclick = function () { pickSection(mi.key); };
          ditems.push({ key: mi.key, node: it });
          draw.appendChild(it);
        })(MENU[m]);
      }
    }
    buildDrawer(false);

    // --- catalog helpers ---
    function item(n) {
      var a = cat && cat.items || [], i;
      for (i = 0; i < a.length; i++) if (a[i].name === n) return a[i];
      return null;
    }
    function title(n) { var it = item(n); return it && it.title || n; }
    function online() { return !!(cat && cat.online); }
    function linked() { return !!(cat && cat.account && cat.account.linked); }

    // --- screen stack ---
    function top() { return stack[stack.length - 1]; }
    function push(s) { top().idx = idx; s.idx = -1; stack.push(s); idx = -1; show(true); }
    // key 8: modal, drawer, then the stack; false at a section root (the host closes the overlay)
    function pop() {
      if (modal) { if (!modal.busy) closeModal(); return true; }
      if (drawerOpen) { setDrawer(false); return true; }
      if (stack.length > 1) { leavePair(); stack.pop(); idx = top().idx; show(true); return true; }
      if (pending.length) { openRestart(); return true; }   // no leaving the store with a restart pending
      return false;
    }
    function pickSection(key) {
      leavePair();
      section = key;
      stack = [{ kind: "root", idx: -1 }];
      fire("menu", "select", key);
      setDrawer(false);
      idx = -1;
      show(true);
    }
    function setDrawer(on) {
      if (on === drawerOpen) return;
      drawerOpen = on;
      draw.className = "rvp-chrome rvp-draw" + (on ? " rvp-on" : "");
      burger.innerHTML = svg(on ? "dots" : "menu");
      scrim.className = "rvp-chrome rvp-scrim" + (on ? " rvp-on" : "");
      fire("drawer", on ? "open" : "close", null);
      if (on) { top().idx = idx; for (var i = 0; i < ditems.length; i++) if (ditems[i].key === section) idx = i; }
      else idx = top().idx;
      paint();
    }
    burger.onclick = function () { setDrawer(!drawerOpen); };
    scrim.onclick = function () { setDrawer(false); };

    // --- store actions --------------------------------------------------------
    function act(name, action, value) {
      topMsg = "";
      delete errs[name];
      store(action, value || { name: name });
      show();
    }
    // path = a USB file from the last scan (plan-usb), else a catalog package
    function usbVal(u) { return u.unsigned ? { path: u.path, unsigned: true } : { path: u.path }; }   // unsigned = consent given
    function plan(name, op, u) {
      asked = { name: name, op: op };
      topMsg = "";
      delete errs[name];
      if (u) store("plan-usb", usbVal(u));
      else store(op === "update" ? "plan-update" : "plan", { name: name });
      show();
    }
    function openPage(name) { push({ kind: "page", name: name, shot: 0 }); }
    function openConfig(name) {
      var v = item(name).config.values, c = {}, k;
      for (k in v) c[k] = v[k];
      push({ kind: "config", name: name, vals: c, edit: -1 });
    }
    function openUsbPage(u) { push({ kind: "page", name: u.name, usb: u, shot: 0 }); }
    function openUsb() { push({ kind: "usb", items: null }); store("usb-scan", null); }
    function openPair() { push({ kind: "pair", frame: null, node: null }); store("pair-start", null); }
    function stopPair() {
      if (pairT.stop) pairT.stop();
      if (pairT.cd) clearTimeout(pairT.cd);
      pairT = {};
    }
    function leavePair() { if (top().kind === "pair") { stopPair(); store("pair-cancel", null); } }
    function setText(node, t) { node.innerHTML = ""; node.appendChild(document.createTextNode(t)); }
    // "Expires in m:ss"; at 0 ask the daemon for a new code (its pair frame restarts all this)
    function countdown(s, secs) {
      var t0 = Date.now();
      secs = +secs > 0 ? +secs : 600;   // note: never 0, or expiry would re-ask in a loop
      function tick() {
        var left = secs - Math.floor((Date.now() - t0) / 1000);
        pairT.cd = null;
        if (left <= 0) { setText(s.exp, "Expires in 0:00"); store("pair-start", null); return; }
        setText(s.exp, "Expires in " + Math.floor(left / 60) + ":" + (left % 60 < 10 ? "0" : "") + left % 60);
        pairT.cd = setTimeout(tick, 1000 - (Date.now() - t0) % 1000);
      }
      tick();
    }

    // --- assets: asked once, filled in place when the frame comes -------------------
    function asset(node, name, kind, i) {
      var k = name + "/" + kind + "/" + i;
      (assetNode[k] = assetNode[k] || []).push(node);   // page thumb + viewer can wait on the same shot
      if (assets[k]) fill(node, assets[k]);
      else if (!req[k]) { req[k] = 1; store("asset", { name: name, kind: kind, i: i }); }
      return k;
    }
    function fill(node, a) {
      if (a.src) { node.innerHTML = ""; node.appendChild(el("img", "rvp-img")).src = a.src; }
      else if (node.className.indexOf("rvp-grey") < 0) node.className += " rvp-grey";
    }
    function icon(it) {
      var ico = el("div", "rvp-ico");
      ico.innerHTML = svg("pkg");
      if (it.icon) asset(ico, it.name, "icon", 0);
      return ico;
    }

    // --- progress (card subtitle / page box), updated in place per frame ---
    function bar(box, p) {
      var b = box.querySelectorAll(".rvp-bar")[0];
      if (!b) { b = box.appendChild(el("div", "rvp-bar")); b.appendChild(el("div", "rvp-fill")); }
      b.children[0].style.width = p.pct + "%";
    }
    function progText(p) { return (STAGE[p.stage] || "Working") + "\u2026 " + p.pct + "%"; }

    // --- views (one per screen kind) --------------------------------------------
    function rating(cls, it, more) {
      var r = el("div", cls);
      r.appendChild(stars(it.rating.avg, 5));
      r.appendChild(document.createTextNode(String(it.rating.avg) + more));
      return r;
    }
    // catalog item, or a USB item (has `path`: manifest fields only)
    function card(it) {
      var cd = el("div", "rvp-card rvp-act"), n = it.name;
      cd.appendChild(icon(it));
      cd.appendChild(el("div", "rvp-nm", it.title || n));
      var sub = el("div", "rvp-sub" + (errs[n] ? " rvp-err" : ""), errs[n] || (prog[n] ? progText(prog[n]) : it.subtitle || (it.path && it.version ? "v" + it.version : "")));
      subs[n] = sub;
      cd.appendChild(sub);
      if (prog[n] && !errs[n]) bar(cd, prog[n]);
      if (it.rating) cd.appendChild(rating("rvp-stars", it, ""));
      var st = it.unsigned ? "warn" : it.state === "update" ? "upd" : it.state === "installed" ? "info" : "";   // no text, an icon
      if (st) cd.appendChild(el("div", "rvp-bi rvp-bi-" + st)).innerHTML = svg(st);
      cd.onclick = function () { if (it.path) openUsbPage(it); else openPage(n); };
      return cd;
    }
    // filter chips on every list: All / Installed (incl. updates) / Available
    function chips(list) {
      var row = list.appendChild(el("div", "rvp-chips")), f = [["", "All"], ["installed", "Installed"], ["available", "Available"]], i;
      for (i = 0; i < f.length; i++) row.appendChild(btn(f[i][1], (function (k) { return function () { filt = k; show(true); }; })(f[i][0]), filt !== f[i][0])).className += " rvp-chip";
    }
    // never-linked unit with the store server unreachable: how to get the head unit online (labels as the stock Settings shows them)
    function offlineHelp(bare) {
      var d = el("div", bare ? "" : "rvp-off"), l, i, st = [
        "Turn on the hotspot on your phone, or make sure a 2.4 GHz Wi-Fi network is in range.",
        "From the Home screen open Settings and go to the Devices tab.",
        "Select Wi-Fi, turn Wi-Fi on and select your network.",
        "Enter the network password and connect.",
        "Make sure the Wi-Fi icon shows in the top bar."];
      d.appendChild(el("div", "rvp-h2", "Please connect CMU to the internet"));
      d.appendChild(el("div", "rvp-line", "To do so, follow these steps:"));
      l = d.appendChild(el("ol", "rvp-offl"));
      for (i = 0; i < st.length; i++) l.appendChild(el("li", "rvp-offs", st[i]));
      d.appendChild(el("div", "rvp-line", "The store online status is shown in the top bar next to the clock."));
      return d;
    }
    function listView() {
      var list = el("div", "rvp-list"), a = cat.items || [], i, n = 0, it;
      chips(list);
      for (i = 0; i < a.length; i++) {
        it = a[i];
        if (it.devOnly || (it.pinned ? "Apps" : SECTION_OF[it.category] || "Apps") !== section) continue;
        if (filt && (it.state === "available") !== (filt === "available")) continue;
        list.appendChild(card(it)); n++;
      }
      if (!n) list.appendChild(el("div", "rvp-empty", "Nothing here yet."));
      return list;
    }
    // Home: sign-in banner (unlinked), Featured cards in `featured` order, News newest first
    function homeView() {
      var p = el("div", "rvp-list"), f = cat.featured || [], news = [], feat = [], i, it, ni, bn;
      if (!linked()) {
        bn = p.appendChild(el("div", "rvp-banner rvp-act"));
        bn.appendChild(el("div", "rvp-nm", "Sign in to install"));
        bn.appendChild(el("div", "rvp-sub", "Link this car to install and update packages"));
        bn.onclick = openPair;
      }
      for (i = 0; i < f.length; i++) { it = item(f[i]); if (it && it.name !== PINNED && !it.devOnly) feat.push(it); }
      if (feat.length) p.appendChild(el("div", "rvp-h2", "Featured"));
      for (i = 0; i < feat.length; i++) p.appendChild(card(feat[i]));
      for (i = 0; i < (cat.news || []).length; i++) news.push({ n: cat.news[i], i: i });
      // note: dates compare as strings (ISO yyyy-mm-dd); index keeps the sort stable
      news.sort(function (a, b) { var x = String(a.n.date || ""), y = String(b.n.date || ""); return x < y ? 1 : x > y ? -1 : a.i - b.i; });
      if (news.length) p.appendChild(el("div", "rvp-h2", "News"));
      for (i = 0; i < news.length; i++) {
        it = news[i].n;
        ni = p.appendChild(el("div", "rvp-news"));
        ni.appendChild(el("div", "rvp-sub", it.date || ""));
        ni.appendChild(el("div", "rvp-nm", it.title || ""));
        ni.appendChild(el("div", "rvp-p", it.text || ""));
        if (it.pkg && item(it.pkg)) {
          ni.className += " rvp-act";
          ni.onclick = (function (pk) { return function () { openPage(pk); }; })(it.pkg);
        }
      }
      return p;
    }
    function accountView() {
      var p = el("div", "rvp-pane"), b, ok;
      if (linked()) {
        p.appendChild(el("div", "rvp-h2", "Linked to " + (cat.account.name || "")));
        b = p.appendChild(el("div", "rvp-btns"));
        b.appendChild(btn("Sign out", function () { store("signout", null); }, true));
      } else {
        p.appendChild(el("div", "rvp-h2", "This car is not linked"));
        p.appendChild(el("div", "rvp-p", "Link it to install and update packages from the store."));
        b = p.appendChild(el("div", "rvp-btns"));
        b.appendChild(btn("Link this car", openPair));
      }
      b = p.appendChild(el("div", "rvp-btns"));
      b.appendChild(btn("Install from USB", openUsb, true));
      if (cat.devmode) p.appendChild(el("div", "rvp-line", "Developer mode: ON"));
      else {
        if (cat.support === "lost") p.appendChild(el("div", "rvp-line", "Brick support: OFF (Developer mode was used). Reflash + clean install to restore."));
        ok = linked() && online();
        b.appendChild(btn("Enable Developer mode", ok ? function () { openModal({ kind: "devmode" }, 0); } : null, true));
        if (!ok) p.appendChild(el("div", "rvp-line", "Link this car and go online first"));
      }
      // versions: installed cmu-re-store package, the SDK API it runs on, the daemon binary inside it
      var hmi = window.RevisedHMI;
      p.appendChild(el("div", "rvp-line", "Store v" + (cat.store || "-") + "   SDK API " + (hmi && hmi.apiVersion || "-") +
        "   Daemon v" + (cat.daemon || "-")));
      return p;
    }
    // modal "ask": head + lines (+ list), then a drag gate (gate label) or Cancel + ok; fn runs on agree / ok
    function ask(h, lines, ok, fn, gate) { openModal({ kind: "ask", h: h, lines: lines, ok: ok, fn: fn, gate: gate }, 0); }
    function devView() {
      var p = el("div", "rvp-pane"), tw = cat.tripwire, ssh = cat.ssh, a = cat.items || [], b, i;
      function twRow(sw, label, can) {   // toggle row like a config switch; disabled = dimmed, not focusable
        var on = !!tw[sw], r = p.appendChild(el("div", "rvp-cfg" + (can ? " rvp-act" : " dis")));
        function send() { closeModal(); store("tripwire", { switch: sw, on: !on }); }
        r.appendChild(el("span", "rvp-cfgval rvp-sw" + (on ? " on" : ""))).appendChild(el("span", "rvp-swk"));
        r.appendChild(document.createTextNode(label));
        if (can) r.onclick = on ? function () { ask("Turn off " + label + "?", [], null, send, "Drag to I AGREE"); }
          : function () { ask("Turn on " + label + "?", [], "Turn on", send); };
      }
      if (tw) {
        p.appendChild(el("div", "rvp-h2", "Tripwire"));
        twRow("nor", "NOR flip", !!tw.all);   // NOR flip only acts while All recovery is on
        twRow("all", "All recovery", true);
        p.appendChild(el("div", "rvp-line", "Takes effect after restart"));
      }
      if (ssh) {
        p.appendChild(el("div", "rvp-h2", "SSH"));
        if (errs[SSH_PKG]) p.appendChild(el("div", "rvp-msg", errs[SSH_PKG]));
        b = p.appendChild(el("div", "rvp-btns"));
        if (ssh.on) {
          p.appendChild(el("div", "rvp-line", ssh.ip ? "ssh -i cmu_key -p 36000 cmu@" + ssh.ip : "Not connected to WiFi"));
          b.appendChild(btn("Disable SSH", function () { act(SSH_PKG, "uninstall"); }, true));
        } else {
          b.appendChild(btn("Enable SSH", !ssh.key ? null : function () {
            ask("Enable SSH?", ["SSH gives root access to anyone holding the key, over the same WiFi network."], "Enable SSH",
              function () { closeModal(); store("ssh-enable", null); });
          }));
          if (!ssh.key) {
            p.appendChild(el("div", "rvp-line", "Make a key on your computer: ssh-keygen -t ecdsa -b 256 -f cmu_key"));
            p.appendChild(el("div", "rvp-line", "Copy cmu_key.pub to the USB stick root as .cmu-re_ssh-publickey"));
          }
        }
      }
      // devOnly packages live only here (the SSH block covers its own package)
      for (i = 0, b = null; i < a.length; i++) {
        if (!a[i].devOnly || a[i].name === SSH_PKG) continue;
        if (!b) { p.appendChild(el("div", "rvp-h2", "Developer packages")); b = p.appendChild(el("div")); }
        b.appendChild(card(a[i]));
      }
      p.appendChild(el("div", "rvp-h2", "Unsigned packages"));
      p.appendChild(el("div", "rvp-btns")).appendChild(btn("Install from USB", openUsb, true));
      p.appendChild(el("div", "rvp-h2", "Leave"));
      p.appendChild(el("div", "rvp-btns")).appendChild(btn("Leave Developer mode", openLeave, true));
      return p;
    }
    // what devmode-leave undoes (daemon order: ssh, devOnly + unsigned packages, tripwire switches)
    function openLeave() {
      var a = cat.items || [], tw = cat.tripwire, ssh = cat.ssh, l = [], i;
      if (ssh && ssh.on) l.push(["SSH (" + SSH_PKG + ")"]);
      for (i = 0; i < a.length; i++) {
        if (a[i].state === "available") continue;
        if (a[i].unsigned) l.push(["Unsigned package: " + a[i].name]);
        else if (a[i].devOnly && a[i].name !== SSH_PKG) l.push(["Developer package: " + a[i].title]);
      }
      if (tw && !tw.all) l.push(["Tripwire All recovery: back ON"]);
      if (tw && !tw.nor) l.push(["Tripwire NOR flip: back ON"]);
      if (l.length) l.unshift("This will undo:");
      l.push("Brick support stays OFF. Only a full firmware reflash (.up) and a clean store install restore it.");
      if (ssh && ssh.ever) l.push("SSH was enabled: anyone with the key may have changed files the store does not track.");
      ask("Leave Developer mode?", l, "Leave", function () { modal.busy = true; modal.err = ""; store("devmode-leave", null); showModal(); paint(); });
      modal.leave = true;
    }
    function usbView(s) {
      var list = el("div", "rvp-list"), a = s.items, i;
      if (!a) list.appendChild(el("div", "rvp-empty", "Looking for packages on USB..."));
      else if (!a.length) list.appendChild(el("div", "rvp-empty", "No packages found on USB"));
      else for (i = 0; i < a.length; i++) list.appendChild(card(a[i]));
      return list;
    }
    function pane(h, t) {
      var p = el("div", "rvp-pane");
      p.appendChild(el("div", "rvp-h1", h));
      if (t) p.appendChild(el("div", "rvp-p", t));
      return p;
    }
    function btn(label, fn, ghost) {
      var b = el("div", "rvp-btn" + (!fn ? " dis" : (ghost ? " ghost" : "") + " rvp-act"), label);
      if (fn) b.onclick = fn;
      return b;
    }
    function permLine(it) {
      var p = it.permissions, w = [], i;
      if (!p) return "Permissions: not declared";
      for (i = 0; i < p.length; i++) w.push(PERM_WORDS[p[i]] || p[i]);
      return "Permissions: " + (w.length ? w.join(", ") : "None");
    }
    // buttons by state; Install / Update need online + a linked account, Remove never is gated
    function buttons(it) {
      var r = el("div", "rvp-btns"), n = it.name;
      if (it.path) {   // USB: no link needed (the daemon asks for one only to fetch dependencies)
        if (it.pinned) r.appendChild(btn("Install with the update stick", null));
        else r.appendChild(btn("Install", it.unsigned && !(cat && cat.devmode) ? null : function () { plan(n, "install", it); }));
        return r;
      }
      function gated(label, op) {
        if (!online()) return btn("Offline", null);
        if (!linked()) return btn("Sign in to " + op, openPair);
        return btn(label, function () { plan(n, op); });
      }
      if (it.pinned) {   // the store: no Remove; the daemon refuses its ops (no OTA yet)
        if (it.state === "update") r.appendChild(btn("Update with the update stick", null));
        return r;
      }
      if (it.state === "available") r.appendChild(gated("Install", "install"));
      if (it.state === "update") r.appendChild(gated("Update", "update"));
      // locked: an installed package needs it (sys-time, sys-trust); the daemon refuses the uninstall too
      if (it.state !== "available" && it.config) r.appendChild(btn("Configuration", function () { openConfig(n); }));
      if (it.state !== "available" && !it.locked) r.appendChild(btn("Remove", function () { act(n, "uninstall"); }, true));
      return r;
    }
    // package settings: one row per field; select/switch change on Enter, range/number on the
    // knob while in edit mode (Enter toggles it); Apply sends the whole set once something changed
    function cfgText(f, v) {
      var i;
      if (f.type === "select") for (i = 0; i < f.options.length; i++) if (f.options[i].value === v) return f.options[i].label;
      return String(v);
    }
    function cfgStep(s, d, i) {   // i: field index, default = the row in edit mode
      var f = item(s.name).config.fields[i == null ? s.edit : i], st = f.type === "range" ? f.step : 1;
      s.vals[f.key] = Math.max(f.min, Math.min(f.max, s.vals[f.key] + d * st));
      show();
    }
    function configView(s) {
      var it = item(s.name), c = it.config, p = el("div", "rvp-page"), changed = false, i;
      p.appendChild(el("div", "rvp-ptitle", (it.title || it.name) + " - Config"));
      if (errs[s.name]) p.appendChild(el("div", "rvp-msg", errs[s.name]));
      for (i = 0; i < c.fields.length; i++) {
        (function (f, i) {
          var r = p.appendChild(el("div", "rvp-cfg rvp-act" + (s.edit === i ? " rvp-edit" : "")));
          if (f.type === "switch") r.appendChild(el("span", "rvp-cfgval rvp-sw" + (s.vals[f.key] ? " on" : ""))).appendChild(el("span", "rvp-swk"));   // toggle: track + knob
          else if (f.type === "number") {   // input group: [-] value [+]; a tap steps by 1, no edit mode
            var g = r.appendChild(el("span", "rvp-cfgval rvp-num"));
            g.appendChild(el("span", "rvp-numb", "-")).onclick = function (e) { e.stopPropagation(); cfgStep(s, -1, i); };
            g.appendChild(el("span", "rvp-numv", String(s.vals[f.key])));
            g.appendChild(el("span", "rvp-numb", "+")).onclick = function (e) { e.stopPropagation(); cfgStep(s, 1, i); };
          } else r.appendChild(el("span", "rvp-cfgval", cfgText(f, s.vals[f.key])));
          r.appendChild(document.createTextNode(f.label));
          if (f.type === "range") {   // slider: track filled to the value + thumb
            var tr = r.appendChild(el("div", "rvp-rng")), pc = Math.round((s.vals[f.key] - f.min) * 100 / (f.max - f.min)) + "%";
            tr.appendChild(el("div", "rvp-rngf")).style.width = pc;
            tr.appendChild(el("span", "rvp-rngk")).style.left = pc;
          }
          r.onclick = function () {
            var o = f.options, j;
            if (f.type === "select") {   // own screen: a long list (timezones) must not cycle
              for (j = 0; j < o.length && o[j].value !== s.vals[f.key]; j++) {}
              push({ kind: "pick", name: s.name, cfg: s, f: f });
              idx = hdrN + (j < o.length ? j : 0); paint();
              return;
            }
            if (f.type === "switch") s.vals[f.key] = !s.vals[f.key];
            else s.edit = s.edit === i ? -1 : i;
            show();
          };
          if (s.vals[f.key] !== c.values[f.key]) changed = true;
        })(c.fields[i], i);
      }
      p.appendChild(el("div", "rvp-btns")).appendChild(btn("Apply", changed ? function () {
        s.edit = -1;
        act(s.name, "config-set", { name: s.name, values: s.vals });
      } : null));
      return p;
    }
    function pickView(s) {   // select options; Enter picks and returns, Back leaves the value
      var p = el("div", "rvp-page"), o = s.f.options, i;
      p.appendChild(el("div", "rvp-ptitle", s.f.label));
      for (i = 0; i < o.length; i++) {
        (function (v) {
          var r = p.appendChild(el("div", "rvp-cfg rvp-act"));
          if (v === s.cfg.vals[s.f.key]) r.appendChild(el("span", "rvp-cfgval", "Selected"));
          r.appendChild(el("span", "rvp-optl", o[i].label));
          r.onclick = function () { s.cfg.vals[s.f.key] = v; pop(); };
        })(o[i].value);
      }
      return p;
    }
    function shotRow(s, it) {   // page: every screenshot as a thumbnail; tap/Enter opens the viewer on it
      var row = el("div", "rvp-shots"), i;
      for (i = 0; i < it.shots; i++) {
        (function (n) {
          var b = row.appendChild(el("div", "rvp-shot rvp-act"));
          asset(b, it.name, "shot", n);
          b.onclick = function () { s.shot = n; openModal({ kind: "shot", s: s, it: it }, 0); };
        })(i);
      }
      return row;
    }
    function shotStrip(s, it) {   // full-screen viewer: one shot + counter; knob/tilt/tap flip
      var strip = el("div", "rvp-shots rvp-act"), box = strip.appendChild(el("div", "rvp-shot"));
      var cnt = strip.appendChild(el("div", "rvp-cnt")), k = null;
      function showShot() {
        var w = k && assetNode[k], j;   // a late frame of the old shot must not fill this box
        if (w) for (j = w.length - 1; j >= 0; j--) if (w[j] === box) w.splice(j, 1);
        box.className = "rvp-shot"; box.innerHTML = "";
        cnt.innerHTML = ""; cnt.appendChild(document.createTextNode((s.shot + 1) + "/" + it.shots));
        k = asset(box, it.name, "shot", s.shot);
      }
      strip.rvpFlip = function (d) {
        var i = s.shot + d;
        if (i >= 0 && i < it.shots) { s.shot = i; showShot(); }
      };
      strip.onclick = function () { strip.rvpFlip(s.shot + 1 < it.shots ? 1 : -s.shot); };
      showShot();
      return strip;
    }
    function sizeText(b) { return b >= 1048576 ? (b / 1048576).toFixed(1) + " MB" : b >= 1024 ? Math.round(b / 1024) + " KB" : b + " B"; }
    // one stats cell: value over label
    function stat(row, val, label) {
      var c = row.appendChild(el("div", "rvp-stat" + (row.children.length ? "" : " first")));
      if (typeof val === "string") c.appendChild(el("div", "rvp-stv", val)); else c.appendChild(val);
      c.appendChild(el("div", "rvp-stl", label));
    }
    // accordion: header (knob-focusable) + body; open state kept on the screen entry
    function accordion(box, s, key, label, body) {
      var w = box.appendChild(el("div", "rvp-acci")), h = w.appendChild(el("div", "rvp-acc rvp-act", label)), ch = h.appendChild(el("span")), b = w.appendChild(body);
      body.className += " rvp-accb";
      function set() { b.style.display = s.acc[key] ? "block" : "none"; ch.innerHTML = svg(s.acc[key] ? "up" : "down"); w.className = "rvp-acci" + (s.acc[key] ? " open" : ""); }
      h.onclick = function () { s.acc[key] = !s.acc[key]; set(); };
      set();
    }
    // package page, laid out like a phone store listing
    function pageView(s) {
      var it = s.usb || item(s.name), n = it.name, p = el("div", "rvp-page"), h = p.appendChild(el("div", "rvp-phead"));
      var d, i, row, v, about, news, chg, files, rate, accs;
      s.acc = s.acc || { about: true, news: false };
      h.appendChild(icon(it));
      h.appendChild(el("div", "rvp-ptitle", it.title || n));
      if (it.author) h.appendChild(el("div", "rvp-pauth", it.author));
      row = h.appendChild(el("div", "rvp-stats"));
      if (it.size) stat(row, sizeText(it.size), "Size");
      if (it.version) stat(row, "v" + it.version, it.state === "update" && it.installed ? "You have v" + it.installed : "Version");
      // rating next to the version: average + stars + votes, then Rate (own stars once rated)
      rate = linked() && it.everInstalled;
      if (it.rating || rate) {
        v = el("div", "rvp-stv", it.rating ? String(it.rating.avg) : "-");
        v.appendChild(stars(it.rating ? it.rating.avg : 0, 5));
        stat(row, v, it.rating ? it.rating.count + (it.rating.count === 1 ? " review" : " reviews") : "No reviews");
      }
      if (rate) {
        d = row.appendChild(btn(it.myStars ? "" : "Rate", online() ? function () { openModal({ kind: "rate", name: n }, it.myStars ? it.myStars - 1 : 0); } : null));
        d.className += " rvp-rbtn";
        if (it.myStars) { d.className += " ghost"; d.appendChild(stars(it.myStars, 5)); }   // red stars vanish on the red button
      }
      if (it.unsigned) h.appendChild(el("div", "rvp-badge", "UNSIGNED"));
      if (errs[n]) p.appendChild(el("div", "rvp-msg", errs[n]));
      if (prog[n]) {   // progress replaces the buttons
        d = p.appendChild(el("div", "rvp-pgbox"));
        subs[n] = d.appendChild(el("div", "rvp-sub", progText(prog[n])));
        bar(d, prog[n]);
      } else p.appendChild(buttons(it));
      if (it.shots > 0) p.appendChild(shotRow(s, it));
      about = el("div");
      d = (it.description || "").split("\n");
      for (i = 0; i < d.length; i++) if (d[i]) about.appendChild(el("div", "rvp-p", d[i]));
      about.appendChild(el("div", "rvp-line", permLine(it)));
      accs = p.appendChild(el("div", "rvp-accs"));
      accordion(accs, s, "about", "About", about);
      // `changes`: the release notes, newest first ("2.0.0: ...")
      if (it.changes && it.changes.length) {
        news = el("div", "rvp-chg");
        for (i = 0; i < it.changes.length; i++) news.appendChild(el("div", null, it.changes[i]));
        accordion(accs, s, "news", "What's new", news);
      }
      // what it does to the unit: relations + the stock files it overrides (installed: registry;
      // USB: manifest). ponytail: catalog entries carry no file list yet; add one to the api release if wanted
      files = it.files || [];
      if (it.path) { files = []; for (i = 0; i < (it.overrides || []).length; i++) files.push(it.overrides[i].live); }
      if (it.depends || it.conflicts || files.length) {
        chg = el("div", "rvp-chg");
        if (it.depends) chg.appendChild(el("div", "rvp-line", "Requires: " + it.depends));
        if (it.conflicts) chg.appendChild(el("div", "rvp-line", "Conflicts: " + it.conflicts));
        if (files.length) chg.appendChild(el("div", "rvp-line", "Overrides " + files.length + (files.length === 1 ? " stock file:" : " stock files:")));
        for (i = 0; i < files.length; i++) chg.appendChild(el("div", null, files[i]));
        accordion(accs, s, "chg", "What it changes", chg);
      }
      return p;
    }
    function verLine(n) {
      var it = item(n);
      if (!it) return n;
      return n + " " + (it.state === "update" && it.installed ? it.installed + " -> " + it.version : it.version);
    }
    // confirm: everything the daemon's plan will fetch, plus the permissions
    function confirmView(s) {
      var verb = s.op === "update" ? "Update" : "Install", it = s.usb || item(s.name);
      var p = pane(verb + " " + (it && it.title || s.name) + "?", "These packages will be installed:"), i, b, o;
      for (i = 0; i < s.list.length; i++) p.appendChild(el("div", "rvp-li", verLine(s.list[i])));
      if (s.usb && s.usb.unsigned) p.appendChild(el("div", "rvp-msg", "NOT SIGNED. No guarantees. A bad package can brick the unit."));
      if (it) p.appendChild(el("div", "rvp-line", permLine(it)));
      o = s.usb && s.usb.overrides || [];
      if (o.length) p.appendChild(el("div", "rvp-line", "Overrides:"));
      for (i = 0; i < o.length; i++) p.appendChild(el("div", "rvp-line", o[i].live + " (" + o[i].type + "): " + o[i].reason));
      if (cat && cat.tripwire && !cat.tripwire.all) p.appendChild(el("div", "rvp-msg", TW_OFF));
      b = p.appendChild(el("div", "rvp-btns"));
      b.appendChild(btn(verb, function () {
        pop();
        if (s.usb) act(s.name, "install-usb", usbVal(s.usb)); else act(s.name, s.op);
      }));
      b.appendChild(btn("Cancel", pop, true));
      return p;
    }
    // Pair: canvas badge on the left (drawQr), DOM text column from x 440; built once per pair
    // frame and kept on the screen so re-renders do not restart the animation
    function pairView(s) {
      if (s.node) return s.node;
      var p = el("div", "rvp-pair"), f = s.frame, box;
      s.cv = p.appendChild(el("canvas", "rvp-qr"));
      s.cv.width = 800; s.cv.height = 432;
      p.appendChild(el("div", "rvp-pt rvp-pt1", "Link this car"));
      p.appendChild(el("div", "rvp-pt rvp-pt2", "Scan the code with your phone"));
      p.appendChild(el("div", "rvp-pt rvp-pt3", "or open this address:"));
      p.appendChild(el("div", "rvp-pt rvp-purl", f ? String(f.url || "").replace(/^https?:\/\//, "") : ""));
      box = p.appendChild(el("div", "rvp-pbox"));
      box.appendChild(el("div", "rvp-plab", "Code"));
      box.appendChild(el("div", "rvp-pcode", f ? f.code || "" : ""));
      s.exp = p.appendChild(el("div", "rvp-pt rvp-pexp", f ? "" : "Requesting a code..."));
      p.appendChild(el("div", "rvp-pt rvp-phint", "Back cancels"));
      s.node = p;
      return p;
    }
    function screenView(s) {
      if (s.kind === "page") return pageView(s);
      if (s.kind === "config") return configView(s);
      if (s.kind === "pick") return pickView(s);
      if (s.kind === "confirm") return confirmView(s);
      if (s.kind === "pair") return pairView(s);
      if (s.kind === "usb") return usbView(s);
      if (!cat) return el("div", "rvp-empty", "No catalog yet");
      if (section === "Account") return accountView();
      if (section === "Developer") return devView();
      return section === "Home" ? homeView() : listView();
    }

    function header() {
      slot.innerHTML = "";
      slot.appendChild(el("span", "rvp-online" + (online() ? " on" : "")));
      if (cat && cat.devmode) slot.appendChild(el("span", "rvp-dev", "DEV"));
      if (pending.length) slot.appendChild(el("span", "rvp-pend rvp-act", "Restart pending")).onclick = openRestart;
      if (cat && !online()) slot.appendChild(el("span", "rvp-pend rvp-act", "Connect to the internet")).onclick = function () { openModal({ kind: "offline" }, 0); };
    }
    // full re-render of the top screen; fresh = a new screen (scrolls to the top)
    function show(fresh) {
      var i, dev = !!(cat && cat.devmode);
      if (cat && dev !== drawDev) buildDrawer(dev);
      if (cat && !dev && section === "Developer") { section = "Account"; stack = [{ kind: "root", idx: -1 }]; idx = -1; fresh = true; }   // left dev mode
      while (stack.length > 1 && (top().kind === "page" && !top().usb && !item(top().name) || (top().kind === "config" || top().kind === "pick") && !(item(top().name) || {}).config)) { stack.pop(); idx = top().idx; fresh = true; }   // package gone
      setTitle("Revised Store - " + section);
      header();
      for (i = 0; i < ditems.length; i++) ditems[i].node.className = "rvp-ditem" + (ditems[i].key === section ? " act" : "");
      container.innerHTML = "";
      subs = {}; assetNode = {};
      if (topMsg) container.appendChild(el("div", "rvp-msg", topMsg));
      if (cat && !online() && !linked() && stack.length === 1 && section !== "Developer") container.appendChild(offlineHelp());
      container.appendChild(screenView(top()));
      if (fresh) { scrollTo(0); scrollBase = 0; }
      paint();
      scrollBase = null;
    }

    // --- modals: restart required, star picker (own focus list) ---
    function openModal(mo, at) {
      if (drawerOpen) setDrawer(false);
      if (!modal) top().idx = idx;
      modal = mo;
      showModal();
      idx = at; paint();
    }
    function openRestart() {
      if (modal && modal.busy) return;   // Restarting...: never swap in a fresh modal (Later would reopen the store)
      openModal({ kind: "restart", busy: false, err: "" }, 1);
    }
    function closeModal() {
      modal = null; showModal(); idx = top().idx; paint();
    }
    function rateFill(k) {
      var a = modalEl.querySelectorAll(".rvp-pstar"), j;
      for (j = 0; j < a.length; j++) a[j].innerHTML = j < k ? STAR_ON : STAR_OFF;
    }
    function showModal() {
      var b, i;
      modalEl.innerHTML = "";
      modalEl.className = "rvp-chrome rvp-modal" + (modal ? " rvp-on" : "");
      mscrim.className = "rvp-chrome rvp-mscrim" + (modal ? " rvp-on" : "");
      if (!modal) return;
      if (modal.kind === "restart") {
        modalEl.appendChild(el("div", "rvp-h1", "Restart required"));
        modalEl.appendChild(el("div", "rvp-p", "Restart the head unit to apply: " + pending.join(", ")));
        if (modal.busy) { modalEl.appendChild(el("div", "rvp-p", "Restarting...")); return; }
        if (modal.err) modalEl.appendChild(el("div", "rvp-msg", modal.err));
        // Later left, Restart right (focus starts on Restart; CCW reaches Later)
        b = modalEl.appendChild(el("div", "rvp-btns"));
        b.appendChild(btn("Later", closeModal, true));
        b.appendChild(btn("Restart", function () { modal.busy = true; modal.err = ""; store("reboot", null); showModal(); paint(); }));
      } else if (modal.kind === "offline") {
        modalEl.appendChild(offlineHelp(true));
        modalEl.appendChild(el("div", "rvp-btns")).appendChild(btn("Close", closeModal));
      } else if (modal.kind === "devmode") {
        modalEl.appendChild(el("div", "rvp-h1", "DEVELOPER MODE"));
        modalEl.appendChild(el("div", "rvp-p", "Developer mode turns off the store's safety checks. Packages and settings you change here can leave the head unit unable to boot."));
        modalEl.appendChild(el("div", "rvp-p", "Bricked units are not supported once Developer mode was on, even after you leave it. Support returns only after a full firmware reflash (.up) and a clean store install."));
        modalEl.appendChild(dragGate("Drag to enable", function () { closeModal(); store("devmode-enable", null); }));
        b = modalEl.appendChild(el("div", "rvp-btns"));
        b.appendChild(btn("Cancel", closeModal, true));
      } else if (modal.kind === "ask") {   // lines: "text" = paragraph, ["text"] = list row
        modalEl.appendChild(el("div", "rvp-h1", modal.h));
        for (i = 0; i < modal.lines.length; i++) {
          if (typeof modal.lines[i] === "string") modalEl.appendChild(el("div", "rvp-p", modal.lines[i]));
          else modalEl.appendChild(el("div", "rvp-li", modal.lines[i][0]));
        }
        if (modal.busy) { modalEl.appendChild(el("div", "rvp-p", "Working...")); return; }
        if (modal.err) modalEl.appendChild(el("div", "rvp-msg", modal.err));
        if (modal.gate) modalEl.appendChild(dragGate(modal.gate, modal.fn));
        b = modalEl.appendChild(el("div", "rvp-btns"));
        b.appendChild(btn("Cancel", closeModal, true));
        if (!modal.gate) b.appendChild(btn(modal.ok, modal.fn));
      } else if (modal.kind === "shot") {   // full-screen screenshot viewer: knob/tap flips, Back closes
        modalEl.className += " rvp-mshot";
        b = modalEl.appendChild(shotStrip(modal.s, modal.it)).appendChild(el("div", "rvp-mx"));
        b.innerHTML = svg("x");
        b.onclick = function (e) { if (e) e.stopPropagation(); closeModal(); };   // tap only: the knob flips here, Back closes
      } else {   // rate: just five stars; focus fills up to the focused one (rateFill), Enter/tap sends it
        modalEl.className += " rvp-mrate";
        for (i = 1; i <= 5; i++) {
          (function (n, name) {
            modalEl.appendChild(el("div", "rvp-pstar rvp-act")).onclick = function () { store("rate", { name: name, stars: n }); closeModal(); };
          })(i, modal.name);
        }
      }
    }

    // --- scrolling: only ctx.scrollTo (SDK) or the viewport's scrollTop (desktop) ----
    var scrollBase = null;   // scroll position just requested by show() (the SDK applies it next frame)
    function scrollTo(y) {
      if (y < 0) y = 0;
      if (ctx) ctx.scrollTo(y);
      else if (viewport) viewport.scrollTop = y;
    }
    function scrollToFocused(n) {
      if (!viewport || !n.getBoundingClientRect) return;
      var r = n.getBoundingClientRect(), cr = container.getBoundingClientRect(), vr = viewport.getBoundingClientRect();
      var y0 = scrollBase !== null ? scrollBase : vr.top - cr.top, vh = viewport.clientHeight;
      var y1 = r.top - cr.top, y2 = r.bottom - cr.top;   // in content coordinates
      if (y1 < y0 + 8) scrollTo(y1 - 8);
      else if (y2 > y0 + vh - 8) scrollTo(y2 - (vh - 8));
    }

    // --- knob: one ordered focus list per screen; drawer and modal have their own ---
    var items = [], idx = -1, hdrN = 0, focused = null;
    function inside(node, root) {
      for (; node; node = node.parentNode) if (node === root) return true;
      return false;
    }
    function acts(root) {
      var nl = root.querySelectorAll(".rvp-act"), a = [], i;
      for (i = 0; i < nl.length; i++) if (nl[i].offsetWidth > 0) a.push(nl[i]);
      return a;
    }
    function collect() {
      var i, a;
      hdrN = 0;
      if (modal) return acts(modalEl);
      if (drawerOpen) { a = []; for (i = 0; i < ditems.length; i++) a.push(ditems[i].node); return a; }
      a = [burger].concat(acts(slot));
      hdrN = a.length;
      return a.concat(acts(container));
    }
    function paint() {
      if (focused) focused.className = focused.className.replace(/\s*rvp-focus\b/g, "");
      focused = null;
      items = collect();
      if (!items.length) { idx = -1; return; }
      // default = first content element; with no content yet the burger shows focus but
      // idx stays -1, so content arriving later (catalog) still gets the default
      if (idx < 0 && items.length > hdrN) {
        idx = hdrN;
        while (idx < items.length - 1 && items[idx].className.indexOf("rvp-chip") >= 0) idx++;   // filter chips: reachable, not the default
      } else if (idx >= items.length) idx = items.length - 1;
      focused = items[at()];
      focused.className += " rvp-focus";
      if (modal && modal.kind === "rate") rateFill(at() + 1);
      if (inside(focused, container)) scrollToFocused(focused);
    }
    function at() { return idx < 0 ? 0 : idx; }
    function move(d) { var i = at() + d; if (i >= 0 && i < items.length) { idx = i; paint(); } }   // clamp, no wrap
    function activate() {
      var f = items[at()];
      if (f && typeof f.onclick === "function") f.onclick();
    }
    // returns true when the key was used
    function keyDown(k) {
      var f = items[at()];
      if (modal && modal.kind === "shot" && (k === 77 || k === 78)) k = k === 77 ? 39 : 37;   // viewer: knob flips
      var cs = !modal && !drawerOpen && top().kind === "config" && top().edit >= 0 ? top() : null;
      if (cs && (k === 77 || k === 78)) { cfgStep(cs, k === 77 ? 1 : -1); fire("knob", "intercept", { key: k }); return true; }   // edit mode: knob = value
      if (cs && k === 8) { cs.edit = -1; show(); fire("knob", "intercept", { key: k }); return true; }
      if (k === 77) move(1);                 // CW
      else if (k === 78) move(-1);           // CCW
      else if (k === 13) activate();         // press
      else if (k === 8) {                    // GoBack / header back
        if (pop()) return true;
        if (!ctx) fire("back", "press", null);   // desktop host closes
        return false;
      } else if ((k === 37 || k === 39) && f && f.rvpFlip) f.rvpFlip(k === 39 ? 1 : -1);   // tilt: screenshot strip only
      else return false;
      fire("knob", "intercept", { key: k });
      return true;
    }
    function swallow(e) {
      if (e.preventDefault) e.preventDefault();
      if (e.stopPropagation) e.stopPropagation();
      if (e.stopImmediatePropagation) e.stopImmediatePropagation();
    }
    function onKey(e) {
      var used = keyDown(e.keyCode);
      if (used) swallow(e);
      return used;
    }
    // keyup only needs hiding from the stock handler (it ends the press it never saw start).
    function onKeyUp(e) {
      var k = e.keyCode;
      if (k === 77 || k === 78 || k === 13 || k === 8 || (k >= 37 && k <= 40)) { swallow(e); return true; }
      return false;
    }
    // window capture runs BEFORE document capture: stock Multicontroller.js listens on
    // document (capture) and was registered first, so a document listener of ours fires
    // after it -> the hidden stock list under the overlay moved/selected (2026-11-12).
    if (bindKeys) {
      window.addEventListener("keydown", onKey, true);
      window.addEventListener("keyup", onKeyUp, true);
    }
    if (ctx) ctx.onKey(function (ev) { return ev.phase === "up" ? onKeyUp({ keyCode: ev.key }) : keyDown(ev.key); });
    // touch: put the selector on the tapped control BEFORE its onclick runs. On the
    // content only: the SDK's scrollport capture swallows the click that ends a drag first.
    function onTap(e) {
      var n, i;
      for (n = e.target; n && n !== container; n = n.parentNode) {
        for (i = 0; i < items.length; i++) if (items[i] === n) { idx = i; paint(); return; }
      }
    }
    if (container.addEventListener) container.addEventListener("click", onTap, true);

    // --- daemon frames (field names from daemon/src/server.rs) -------------
    function onDaemon(msg) {
      if (!msg || !msg.t) return;
      var t = msg.t, n = msg.name, s, k, op, first, i;
      if (t === "catalog") {
        cat = msg;
        if (keepErr) keepErr = false; else errs = {};
        prog = {};
        pending = (msg.restartPending || []).slice(0);
        first = !sawCat; sawCat = true;
        show();
        if (first && pending.length) openRestart();   // reopened with a restart pending
        else if (modal && modal.kind === "restart") { showModal(); paint(); }
        return;
      } else if (t === "offline") {
        cat = null; asked = null;
        topMsg = ""; errs = {}; prog = {}; keepErr = false;
      } else if (t === "plan") {
        if (!asked || n !== asked.name) return;   // another client's plan
        op = asked.op; asked = null;
        if (msg.error) errs[n] = msg.error;       // refused (conflict, cycle, ...): no confirm
        else if (!modal && !drawerOpen && top().kind === "page" && top().name === n) {
          push({ kind: "confirm", name: n, op: op, usb: top().usb, list: msg.install || [] });
          return;
        } else topMsg = (op === "update" ? "Update" : "Install") + " of " + n + " not started: page was closed";
      } else if (t === "progress") {
        first = !prog[n];
        prog[n] = msg;
        s = subs[n];
        if (first || !s) { show(); return; }
        s.className = "rvp-sub"; s.innerHTML = ""; s.appendChild(document.createTextNode(progText(msg)));
        bar(s.parentNode, msg);
        return;                               // in place: no re-render per frame
      } else if (t === "result") {
        delete prog[n];
        if (msg.op === "devmode-leave") {   // failures come as an error frame
          if (modal && modal.leave) closeModal();
          if (msg.restart) { if (pending.indexOf("Developer mode") < 0) pending.push("Developer mode"); openRestart(); }
          return;                           // the catalog that follows moves to Account
        }
        if (!msg.ok) { errs[n] = msg.error || "failed"; keepErr = true; }
        else {
          topMsg = msg.error || (msg.op === "config-set" ? "Settings applied" : RESTART_NOTE);   // ok + error: committed, only the ro remount failed
          if (msg.restart) {
            k = title(n);
            if (pending.indexOf(k) < 0) pending.push(k);   // the catalog that follows brings the full list
            show(); openRestart(); return;
          }
        }
      } else if (t === "error") {
        asked = null;
        if (modal && modal.busy) {   // reboot refused / dbus failed: buttons back
          modal.busy = false; modal.err = msg.error || "error";
          showModal(); idx = 1; paint(); return;
        }
        topMsg = msg.error || "error";
      } else if (t === "pair") {
        s = top();
        if (s.kind !== "pair") return;          // the Pair screen was left
        stopPair();
        s.frame = msg; s.node = null;
        show();
        pairT.stop = drawQr(s.cv, msg.qr || []);
        countdown(s, msg.expiresIn);
        return;
      } else if (t === "pair-done") {
        if (top().kind !== "pair") return;
        stopPair(); stack.pop(); idx = top().idx; show(true);   // the catalog that follows shows the link
        return;
      } else if (t === "usb") {
        if (top().kind !== "usb") return;
        top().items = msg.items || [];
      } else if (t === "asset") {
        k = n + "/" + msg.kind + "/" + msg.i;
        assets[k] = msg;
        if (assetNode[k]) { for (i = 0; i < assetNode[k].length; i++) fill(assetNode[k][i], msg); delete assetNode[k]; }
        return;
      } else return;                            // rebooting: the modal already says so
      show();
    }

    show(true);

    return {
      teardown: function () {
        leavePair();
        try { window.removeEventListener("keydown", onKey, true); } catch (e) {}
        try { window.removeEventListener("keyup", onKeyUp, true); } catch (e) {}
        try { container.removeEventListener("click", onTap, true); } catch (e) {}
        for (var i = 0; i < chrome.length; i++) {
          if (chrome[i].parentNode) chrome[i].parentNode.removeChild(chrome[i]);
        }
      },
      onDaemon: onDaemon,
      key: onKey,
      keyUp: onKeyUp,
      push: push,
      pop: pop
    };
  }

  return { injectStyles: injectStyles, render: render };
})();

// ---- SDK shim: register as the store app (window.__rvpDaemon is defined by the injector) ----
if (window.RevisedHMI && window.RevisedHMI.register) {
  (function () {
    var live = null;   // { ctl, off } of the open overlay
    window.RevisedHMI.register("cmu-re-store/store", {
      open: function (ctx) {
        var d = window.__rvpDaemon, l, ctl;
        RevisedTestApp.injectStyles();
        ctl = RevisedTestApp.render(ctx.root, function (evt) { if (d) d.send(evt); }, { ctx: ctx });
        live = { ctl: ctl, off: d ? d.onFrame(ctl.onDaemon) : null };
        if (d) {
          l = d.last();
          if (l) ctl.onDaemon(l);   // show the cached list at once, then ask for a fresh one
          d.send({ control: "store", action: "catalog", value: null, ts: +new Date() });
        }
        return null;   // no drag hooks: the SDK owns scrolling, the store calls ctx.scrollTo
      },
      close: function () {
        if (!live) return;
        if (live.off) live.off();
        live.ctl.teardown();
        live = null;
      }
    });
  })();
}

// CommonJS escape hatch (harmless in Presto).
if (typeof module !== "undefined" && module.exports) module.exports = RevisedTestApp;
