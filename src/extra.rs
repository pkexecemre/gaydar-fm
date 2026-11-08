//! Additional screen-specific styles, kept separate for readability.

pub const EXTRA_CSS: &str = r#"
.cover-a { background:radial-gradient(circle at 25% 20%,#ffcf5c 0 8%,transparent 9%),linear-gradient(145deg,#ff2d87,#8b3dff 55%,#2a0c36); }
.cover-b { background:radial-gradient(circle at 70% 28%,#7ef9ff 0 9%,transparent 10%),linear-gradient(155deg,#351350,#a13cff 50%,#ff4fa3); }
.cover-c { background:radial-gradient(circle at 30% 72%,#d6ff3f 0 10%,transparent 11%),linear-gradient(145deg,#ffd7d1,#d9a6ff 55%,#6f58c9); }
.cover-d { background:repeating-linear-gradient(90deg,rgba(255,255,255,.12) 0 1px,transparent 1px 12px),linear-gradient(145deg,#d6ff3f,#35dfc9 55%,#4b2bff); }
.cover-e { background:conic-gradient(from 220deg,#ff4fa3,#ffcf5c,#7ef9ff,#b899ff,#ff4fa3); }
.cover-f { background:radial-gradient(circle at 50% 38%,#ff4fa3 0 14%,transparent 15%),linear-gradient(145deg,#1a0b22,#75204f 60%,#ff9d5c); }
.cover-a:after,.cover-b:after,.cover-c:after,.cover-d:after,.cover-e:after,.cover-f:after { content:""; position:absolute; inset:0; background:linear-gradient(160deg,rgba(255,255,255,.18),transparent 45%); }

/* ---------- shared screen bits ---------- */
.screen-title { font-family:var(--display); font-weight:900; font-size:32px; line-height:1.05;
  letter-spacing:-1.1px; }
.screen-lede { margin-top:8px; max-width:520px; font-size:11.5px; line-height:1.55; color:#a994b1; }
.screen-head { display:flex; align-items:flex-end; justify-content:space-between; gap:20px; margin-bottom:18px; }
.serif-accent { font-family:var(--serif); font-style:italic; font-weight:400; color:var(--pink); }
.chip-row { display:flex; flex-wrap:wrap; gap:7px; }
.chip { padding:5px 9px; border:1px solid var(--line); border-radius:5px; font-size:8px;
  letter-spacing:1.1px; text-transform:uppercase; color:#b8a5c0; background:rgba(255,255,255,.025);
  cursor:pointer; transition:all .15s ease; }
.chip:hover { border-color:var(--pink); color:var(--pink); }
.chip.on { background:var(--lime); border-color:var(--lime); color:#111408; font-weight:800; }
.stat-row { display:flex; gap:10px; }
.stat-card { flex:1; min-width:0; padding:12px 13px; border:1px solid var(--line); border-radius:10px;
  background:rgba(255,255,255,.025); }
.stat-num { font-family:var(--display); font-weight:800; font-size:20px; line-height:1; }
.stat-label { margin-top:5px; font-size:7px; letter-spacing:1.5px; color:#816f89; text-transform:uppercase; }

/* ---------- gaydar ---------- */
.gaydar-layout { display:flex; gap:16px; align-items:stretch; }
.radar-card { flex:1; min-width:0; min-height:390px; position:relative; overflow:hidden; border-radius:18px;
  border:1px solid rgba(214,255,63,.2); background:
    radial-gradient(circle at 50% 50%, rgba(214,255,63,.06), transparent 36%),
    linear-gradient(145deg,#0d0711,#1a1022 60%,#121c12); padding:18px; }
.radar-head { display:flex; align-items:center; justify-content:space-between; position:relative; z-index:2; }
.radar-label { font-size:8px; letter-spacing:2px; color:#9fae8e; text-transform:uppercase; }
.radar-live { display:flex; align-items:center; gap:6px; font-size:8px; letter-spacing:1.3px; color:var(--lime); }
.radar-plot { position:relative; width:300px; height:300px; margin:6px auto 0; }
.radar-ring { position:absolute; border-radius:50%; border:1px solid rgba(214,255,63,.16); inset:0; }
.radar-sweep { position:absolute; inset:0; border-radius:50%;
  background:conic-gradient(from 20deg, rgba(214,255,63,0) 0deg, rgba(214,255,63,0) 292deg, rgba(214,255,63,.02) 300deg, rgba(214,255,63,.30) 355deg, rgba(214,255,63,.55) 360deg);
  animation:sweep 5s linear infinite; }
.radar-blip { position:absolute; width:5px; height:5px; border-radius:50%; background:var(--lime);
  box-shadow:0 0 10px rgba(214,255,63,.8); }
.radar-blip.pink { background:var(--pink); box-shadow:0 0 10px rgba(255,79,163,.8); }
.radar-blip.cyan { background:var(--cyan); box-shadow:0 0 10px rgba(126,249,255,.8); }
.radar-center { position:absolute; width:46px; height:46px; border-radius:50%; left:127px; top:127px;
  background:var(--pink); color:#180913; display:flex; align-items:center; justify-content:center;
  font-size:18px; border:1px solid rgba(255,255,255,.5); box-shadow:0 0 0 10px rgba(255,79,163,.08),0 0 34px rgba(255,79,163,.5); }
.radar-note { position:absolute; left:20px; right:20px; bottom:14px; display:flex; align-items:center;
  justify-content:space-between; font-size:8px; color:#7e8d70; letter-spacing:.8px; }
.dials-panel { width:350px; flex:0 0 350px; padding:16px; border:1px solid var(--line); border-radius:18px;
  background:rgba(255,255,255,.025); }
.dials-title { font-family:var(--display); font-size:11px; font-weight:800; letter-spacing:.5px; }
.dials-note { margin-top:4px; font-size:9px; color:#8d7b95; line-height:1.45; }
.dial { margin-top:14px; }
.dial-top { display:flex; align-items:baseline; justify-content:space-between; }
.dial-name { font-size:8px; letter-spacing:1.8px; text-transform:uppercase; color:#c8b7ce; }
.dial-value { font-family:var(--serif); font-style:italic; font-size:19px; color:var(--cream); }
.dial-desc { margin-top:2px; font-size:8px; color:#79677f; }
.dial-segs { display:flex; gap:3px; margin-top:6px; height:10px; }
.dial-seg { flex:1; border-radius:2px; background:rgba(255,255,255,.08); cursor:pointer; transition:background .1s ease; }
.dial-seg:hover { background:rgba(255,255,255,.22); }
.dial-actions { display:flex; gap:7px; margin-top:16px; }
.mini-btn { flex:1; height:30px; border:1px solid var(--line); border-radius:6px; background:rgba(255,255,255,.035);
  color:#d2c0d8; font-size:8px; letter-spacing:1.2px; text-transform:uppercase; cursor:pointer; }
.mini-btn:hover { border-color:var(--pink); color:var(--pink); }
.mini-btn.solid { background:var(--pink); border-color:var(--pink); color:#180913; font-weight:800; }
.taste-card { margin-top:14px; padding:14px; border:1px solid var(--line); border-radius:14px; background:rgba(255,255,255,.02); }
.taste-title { font-size:8px; letter-spacing:1.8px; color:#89778f; text-transform:uppercase; }
.taste-cloud { display:flex; flex-wrap:wrap; gap:6px; margin-top:8px; }
.taste-tag { padding:4px 7px; background:rgba(255,79,163,.1); border:1px solid rgba(255,79,163,.24);
  border-radius:4px; color:#e8b9d0; font-size:8px; }
.taste-tag b { color:var(--pink); font-weight:700; margin-left:3px; }
.gaydar-lower { display:flex; gap:16px; margin-top:16px; }
.reason-card { flex:1; padding:15px 16px; border-radius:14px; border:1px solid rgba(255,207,92,.2);
  background:linear-gradient(135deg,rgba(255,207,92,.08),rgba(255,79,163,.05)); }
.reason-eyebrow { font-size:7px; letter-spacing:1.8px; color:#c7a66a; text-transform:uppercase; }
.reason-text { font-family:var(--serif); font-style:italic; font-size:23px; line-height:1.15; margin-top:5px; color:#ffeccf; }
.reason-track { margin-top:8px; font-size:9px; color:#9d8b94; }

/* ---------- search ---------- */
.search-hero { padding:20px; border-radius:18px; border:1px solid var(--line);
  background:radial-gradient(circle at 92% 18%,rgba(255,79,163,.14),transparent 30%),rgba(255,255,255,.025); }
.search-big { display:flex; gap:8px; margin-top:14px; }
.search-input { flex:1; height:44px; padding:0 14px; border-radius:9px; border:1px solid rgba(255,255,255,.16);
  background:#100916; color:var(--cream); font-size:13px; outline:none; }
.search-input:focus { border-color:var(--pink); box-shadow:0 0 0 3px rgba(255,79,163,.12); }
.search-btn { height:44px; padding:0 18px; border-radius:9px; border:1px solid rgba(255,255,255,.28);
  background:var(--pink); color:#180913; font-size:9px; font-weight:800; letter-spacing:1.4px; text-transform:uppercase; cursor:pointer; }
.search-hint { margin-top:8px; font-size:9px; color:#7c6b83; }
.search-results-head { margin:20px 0 8px; display:flex; align-items:center; justify-content:space-between; }
.result-count { font-size:9px; color:#8c7a93; }

/* ---------- settings ---------- */
.settings-grid { display:flex; gap:16px; align-items:flex-start; }
.settings-main { flex:1; min-width:0; display:flex; flex-direction:column; gap:14px; }
.settings-side { width:330px; flex:0 0 330px; display:flex; flex-direction:column; gap:14px; }
.setting-card { padding:16px; border:1px solid var(--line); border-radius:14px; background:rgba(255,255,255,.025); }
.setting-title { font-family:var(--display); font-size:10px; font-weight:800; letter-spacing:.4px; }
.setting-note { margin-top:4px; font-size:9px; color:#8b7992; line-height:1.5; }
.device-list { display:flex; flex-direction:column; gap:5px; margin-top:11px; max-height:188px; overflow-y:auto; }
.setting-head-row { display:flex; align-items:center; justify-content:space-between; gap:10px; }
.setting-action { font-size:7px; letter-spacing:1.4px; color:#8d7b95; cursor:pointer; padding:3px 6px; border:1px solid var(--line); border-radius:4px; }
.setting-action:hover { color:var(--pink); border-color:var(--pink); }
.device-row { display:flex; align-items:center; gap:9px; padding:8px 9px; border-radius:7px; cursor:pointer;
  border:1px solid transparent; }
.device-row:hover { background:rgba(255,255,255,.04); }
.device-row.on { border-color:rgba(214,255,63,.32); background:rgba(214,255,63,.06); }
.device-dot { width:6px; height:6px; border-radius:50%; background:#56465c; }
.device-row.on .device-dot { background:var(--lime); box-shadow:0 0 8px rgba(214,255,63,.6); }
.device-name { font-size:10px; font-weight:600; }
.device-desc { margin-left:auto; font-size:8px; color:#79677f; max-width:180px; text-align:right; }
.volume-demo { margin-top:10px; display:flex; align-items:center; gap:10px; }
.volume-big { flex:1; height:9px; border-radius:5px; background:rgba(255,255,255,.09); cursor:pointer; }
.volume-big-fill { height:9px; border-radius:5px; background:linear-gradient(90deg,#ff4fa3,#ffcf5c); }
.kv { display:flex; justify-content:space-between; gap:12px; padding:7px 0; border-bottom:1px solid rgba(255,255,255,.05); font-size:9px; }
.kv:last-child { border-bottom:0; }
.kv span:first-child { color:#8b7992; }
.kv span:last-child { color:#e6d7e9; }
.about-line { font-size:9px; line-height:1.65; color:#9c89a3; }
.about-line b { color:var(--cream); }

/* ---------- now playing ---------- */
.np { flex:1; min-height:0; display:flex; flex-direction:column; position:relative; overflow:hidden;
  background:radial-gradient(circle at 22% 62%,rgba(255,79,163,.16),transparent 34%),#0b050d; }
.np-bg { position:absolute; inset:-30px; opacity:.18; filter:blur(70px); background:linear-gradient(130deg,#ff2d87,#5d1e9e 40%,#0b050d 75%); }
.np-top { position:relative; z-index:2; display:flex; align-items:center; gap:14px; padding:17px 26px;
  border-bottom:1px solid var(--line); }
.np-back { font-size:9px; letter-spacing:1.4px; color:#b8a5c0; text-transform:uppercase; cursor:pointer; }
.np-back:hover { color:var(--pink); }
.np-top-title { font-family:var(--display); font-size:10px; font-weight:800; letter-spacing:1.8px; }
.np-top-right { margin-left:auto; font-size:8px; letter-spacing:1.7px; color:#88768f; text-transform:uppercase; }
.np-body { flex:1; min-height:0; display:flex; gap:42px; padding:34px 42px 20px; position:relative; z-index:2; }
.np-left { width:42%; flex:0 0 42%; display:flex; align-items:center; justify-content:center; }
.np-art-wrap { position:relative; width:100%; max-width:360px; aspect-ratio:1; }
.np-vinyl { position:absolute; width:72%; height:72%; border-radius:50%; right:-18%; top:14%;
  background:conic-gradient(from 20deg,#17101b,#2b1731 7%,#17101b 15%,#25142b 23%,#17101b 31%,#2b1731 39%,#17101b 47%,#25142b 55%,#17101b 63%,#2b1731 71%,#17101b 79%,#25142b 87%,#17101b);
  box-shadow:0 25px 60px rgba(0,0,0,.5); animation:spin 26s linear infinite; }
.np-vinyl:after { content:""; position:absolute; inset:31%; border-radius:50%;
  background:radial-gradient(circle,#ffcf5c 0 20%,#ff4fa3 21% 55%,#24102a 56%); }
.np-art { position:absolute; inset:0; border-radius:8px; overflow:hidden; border:1px solid rgba(255,255,255,.35);
  box-shadow:0 30px 70px rgba(0,0,0,.55); transform:rotate(-2deg); }
.np-art img { width:100%; height:100%; object-fit:cover; }
.np-art .cover-card { inset:0; width:100%; height:100%; transform:none; border:0; box-shadow:none; }
.np-info { flex:1; min-width:0; display:flex; flex-direction:column; justify-content:center; }
.np-eyebrow { display:flex; align-items:center; gap:8px; font-size:8px; letter-spacing:2.2px; color:#d89fbe; text-transform:uppercase; }
.np-cert { padding:3px 6px; border:1px solid rgba(214,255,63,.35); border-radius:3px; color:var(--lime); letter-spacing:1.2px; }
.np-big { margin-top:12px; font-family:var(--display); font-weight:900; font-size:42px; line-height:1.02;
  letter-spacing:-1.6px; color:var(--cream); }
.np-serif { margin-top:2px; font-family:var(--serif); font-style:italic; font-size:30px; color:#ffd8e8; line-height:1.1; }
.np-track-title { margin-top:23px; font-size:16px; font-weight:700; }
.np-track-artist { margin-top:3px; font-size:11px; color:#ad98b4; }
.np-actions { display:flex; align-items:center; gap:10px; margin-top:22px; }
.np-heart { width:37px; height:37px; border-radius:50%; border:1px solid var(--line); display:flex;
  align-items:center; justify-content:center; color:#b39fba; cursor:pointer; }
.np-heart.on { color:var(--pink); border-color:rgba(255,79,163,.4); background:rgba(255,79,163,.08); }
.np-play { width:54px; height:54px; border-radius:50%; background:var(--pink); color:#1a0913;
  display:flex; align-items:center; justify-content:center; cursor:pointer; box-shadow:0 10px 30px rgba(255,79,163,.35);
  border:1px solid rgba(255,255,255,.4); }
.np-play:hover { transform:scale(1.04); }
.np-seek { margin-top:26px; display:flex; align-items:center; gap:10px; }
.np-seek .seek { height:7px; }
.np-seek .seek-fill { height:7px; }
.np-times { display:flex; justify-content:space-between; margin-top:6px; font-size:8px; color:#8b7892; }
.np-queue { width:300px; flex:0 0 300px; border-left:1px solid var(--line); padding-left:24px;
  display:flex; flex-direction:column; min-height:0; }
.np-queue-title { font-family:var(--display); font-size:10px; font-weight:800; letter-spacing:1.3px; }
.np-queue-sub { margin-top:5px; font-size:8px; letter-spacing:1.4px; color:#7c6b83; text-transform:uppercase; }
.np-queue-list { margin-top:13px; overflow-y:auto; flex:1; min-height:0; }
.np-queue-row { display:flex; gap:9px; align-items:center; padding:7px 0; border-bottom:1px solid rgba(255,255,255,.055); cursor:pointer; }
.np-queue-row:hover .q-title { color:var(--pink); }
.q-art { width:34px; height:34px; flex:0 0 34px; border-radius:4px; overflow:hidden; position:relative; }
.q-art img { width:100%; height:100%; object-fit:cover; }
.q-info { min-width:0; flex:1; }
.q-title { font-size:9px; font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.q-artist { margin-top:2px; font-size:8px; color:#8e7b95; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.q-dur { font-size:8px; color:#6f5e76; }
.np-foot { position:relative; z-index:2; padding:0 42px 17px; display:flex; align-items:center; justify-content:space-between; }
.np-tags { display:flex; gap:6px; }
.np-foot-right { display:flex; align-items:center; gap:10px; }
.np-vol-label { font-size:7px; letter-spacing:1.4px; color:#8d7b95; }
.np-foot-right .volume-bars { width:74px; }
.np-tag { padding:3px 6px; border:1px solid var(--line); border-radius:3px; font-size:7px; letter-spacing:1px; color:#a994b1; text-transform:uppercase; }

/* ---------- boot ---------- */
.boot { height:100vh; display:flex; align-items:center; justify-content:center; position:relative; overflow:hidden;
  background:radial-gradient(circle at 50% 42%,rgba(255,79,163,.20),transparent 26%),#0b050d; }
.boot-radar { width:360px; height:360px; position:relative; border-radius:50%;
  background:radial-gradient(circle,rgba(214,255,63,.07),transparent 68%); }
.boot-title { position:absolute; left:50%; bottom:17%; transform:translateX(-50%); text-align:center; width:520px; }
.boot-kicker { font-size:8px; letter-spacing:3px; color:var(--pink); text-transform:uppercase; }
.boot-big { margin-top:9px; font-family:var(--display); font-weight:900; font-size:36px; letter-spacing:-1.4px; }
.boot-serif { font-family:var(--serif); font-style:italic; font-size:46px; color:#ffd7e8; line-height:1; }
.boot-copy { margin-top:12px; font-size:10px; color:#a993b1; }
.boot-bar { width:250px; height:3px; margin:17px auto 0; border-radius:4px; background:rgba(255,255,255,.1); overflow:hidden; }
.boot-bar-fill { width:34%; height:3px; background:linear-gradient(90deg,#ff4fa3,#ffcf5c,#7ef9ff); animation:shimmer 1.8s ease-in-out infinite; }
.nav-active-mark { margin-left:auto; width:5px; height:5px; border-radius:50%; background:#160812; opacity:.75; }
.hero-eq span { height:15px; }
.sticker-note { margin-top:2px; font-family:var(--body); font-size:6px; letter-spacing:1.7px; }
.btn-primary .ui-icon, .btn-ghost .ui-icon { opacity:.95; }
.track-row.active { background:rgba(255,79,163,.075); border-radius:7px; }
.track-row.active .track-title { color:#ffb2d4; }
.mini-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:12px; }
.mini-card { display:flex; align-items:center; gap:9px; padding:7px; border:1px solid var(--line); border-radius:9px;
  background:rgba(255,255,255,.022); cursor:pointer; transition:background .15s ease, transform .15s ease; }
.mini-card:hover { background:rgba(255,255,255,.05); transform:translateY(-2px); }
.mini-art { width:44px; height:44px; flex:0 0 44px; border-radius:6px; overflow:hidden; position:relative; }
.mini-art img { width:100%; height:100%; object-fit:cover; }
.mini-info { min-width:0; }
.mini-title { font-size:10px; font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.mini-artist { margin-top:3px; font-size:8px; color:#8c7a93; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.empty-inline { padding:15px; border:1px dashed var(--line); border-radius:9px; font-size:10px; color:#8c7a93; }
.empty-kicker { font-size:8px; letter-spacing:2px; color:var(--pink); text-transform:uppercase; margin-bottom:7px; }
.empty-player { margin:auto; text-align:center; }
.searching { padding:26px; display:flex; flex-direction:column; align-items:center; gap:12px; }
.search-pulse { width:120px; height:3px; border-radius:4px; background:linear-gradient(90deg,#ff4fa3,#ffcf5c,#7ef9ff); animation:shimmer 1.4s ease-in-out infinite; }
.searching-text { font-family:var(--serif); font-style:italic; font-size:17px; color:#c9a7c0; }
.radar-axis { position:absolute; background:rgba(214,255,63,.09); }
.radar-axis.x { left:0; right:0; top:149px; height:1px; }
.radar-axis.y { top:0; bottom:0; left:149px; width:1px; }
.radar-plot .radar-ring:nth-child(2) { inset:38px; }
.np-viz { display:flex; align-items:flex-end; gap:2px; height:48px; margin-top:24px; width:100%; }
.seek-row .seek, .np-seek .seek { overflow:visible; }
.search-input::placeholder { color:#796a80; }

.stat-num.artist { font-family:var(--display); font-size:13px; line-height:1.15; letter-spacing:-.4px; padding-top:5px; }
.fancy-search { flex:1; height:44px; display:flex; align-items:center; gap:10px; padding:0 14px;
  border-radius:9px; border:1px solid rgba(255,255,255,.16); background:#100916; color:var(--cream);
  font-size:13px; cursor:text; transition:border-color .16s ease; }
.fancy-search:hover { border-color:rgba(255,79,163,.4); }
.fancy-search:focus { border-color:var(--pink); box-shadow:0 0 0 3px rgba(255,79,163,.12); }
.fancy-search-placeholder { color:#796a80; font-size:13px; }
.fancy-search-value { color:var(--cream); font-size:13px; white-space:pre; overflow:hidden; }
.search-caret { width:1.5px; height:18px; background:var(--pink); animation:caret 1.05s steps(2,start) infinite; margin-left:-4px; }
.fancy-search-clear { margin-left:auto; width:22px; height:22px; border-radius:50%; display:flex;
  align-items:center; justify-content:center; cursor:pointer; }
.fancy-search-clear:hover { background:rgba(255,255,255,.07); }
@keyframes caret { 0%,49% { opacity:1; } 50%,100% { opacity:0; } }
.radar-center { font-size:0; }
.radar-center .ui-icon { margin:0; }
.search-btn { display:flex; align-items:center; justify-content:center; cursor:pointer; background:var(--pink); color:#180913; border:1px solid rgba(255,255,255,.25); }
.search-btn:hover { background:#ff6bb5; }
.dial-whisper { margin-top:12px; padding-top:10px; border-top:1px solid var(--line); }
.dial-whisper-kicker { font-size:7px; letter-spacing:1.9px; color:#827188; text-transform:uppercase; }
.dial-whisper-copy { margin-top:4px; font-family:var(--serif); font-style:italic; font-size:14px; line-height:1.25; color:#c9a7c0; }

/* Boot */
.boot { height:100vh; position:relative; overflow:hidden; display:flex; align-items:center; justify-content:center;
  background:radial-gradient(circle at 50% 46%,rgba(255,79,163,.14),transparent 31%),
             radial-gradient(circle at 50% 46%,rgba(214,255,63,.06),transparent 48%),#0b050d; }
.boot-bgword { position:absolute; left:50%; top:47%; transform:translate(-50%,-50%); white-space:nowrap;
  font-family:var(--display); font-weight:900; font-size:230px; letter-spacing:-14px; color:rgba(255,243,228,.027); }
.boot-orbit { position:absolute; width:560px; height:560px; left:50%; top:50%; transform:translate(-50%,-52%); opacity:.28; }
.boot-center-heart { left:257px; top:257px; }
.boot-center { position:relative; z-index:3; width:620px; text-align:center; padding-bottom:52px; }
.boot-brand { display:flex; align-items:center; justify-content:center; gap:11px; margin-bottom:24px; }
.boot-logo { width:46px; height:46px; flex:0 0 46px; }
.boot-wordmark { font-family:var(--display); font-weight:900; font-size:23px; letter-spacing:-.5px; }
.boot-kicker { font-size:8px; letter-spacing:2.8px; color:var(--pink); text-transform:uppercase; }
.boot-big { margin-top:11px; font-family:var(--display); font-weight:900; font-size:38px; line-height:1.05; letter-spacing:-1.5px; color:var(--cream); }
.boot-serif { margin-top:2px; font-family:var(--serif); font-style:italic; font-size:48px; line-height:1.05; color:#ffd7e8; }
.boot-copy { max-width:430px; margin:12px auto 0; font-size:10px; line-height:1.6; color:#a993b1; }
.boot-bar { width:230px; height:3px; margin:20px auto 0; border-radius:4px; background:rgba(255,255,255,.1); overflow:hidden; }
.boot-bar-fill { width:44%; height:3px; border-radius:4px; background:linear-gradient(90deg,#ff4fa3,#ffcf5c,#7ef9ff); animation:bootshimmer 1.7s ease-in-out infinite; }
.boot-bar-fill.failed { width:100%; background:#ff6b6b; animation:none; }
@keyframes bootshimmer { 0% { transform:translateX(-110%); } 100% { transform:translateX(330%); } }
.boot-steps { display:flex; align-items:center; justify-content:center; gap:16px; margin-top:14px; }
.boot-step { font-size:7px; letter-spacing:1.4px; color:#58495f; }
.boot-step.on { color:var(--pink); }
.boot-step.dim { color:#6e5f75; }
.boot-step.bad { color:#ff6b6b; }
.boot-retry { display:inline-flex; align-items:center; justify-content:center; margin-top:20px; height:38px; padding:0 18px;
  border:1px solid var(--pink); border-radius:8px; background:var(--pink); color:#180913; font-size:9px;
  font-weight:800; letter-spacing:1.5px; cursor:pointer; }
.boot-retry:hover { background:#ff6bb5; }
.boot-foot { position:absolute; bottom:22px; left:0; right:0; text-align:center; font-size:7px;
  letter-spacing:1.8px; color:#5d4c65; text-transform:uppercase; }
.boot-bgword { font-size:80px; letter-spacing:3px; top:12%; color:rgba(255,243,228,.026); }

/* Now Playing extras */
.np-cert { margin-left:12px; padding-left:9px; border-left:1px solid rgba(214,255,63,.3); display:flex; align-items:center; }
.np-reading { margin-top:22px; padding:12px 14px; border:1px solid var(--line); border-radius:10px;
  background:rgba(255,255,255,.025); }
.np-reading-title { font-size:7px; letter-spacing:1.9px; color:#8d7b95; text-transform:uppercase; }
.np-reading-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:12px; margin-top:9px; }
.np-read-item { display:flex; align-items:center; gap:7px; }
.np-read-label { width:30px; flex:0 0 30px; font-size:7px; letter-spacing:1.1px; color:#b8a5c0; }
.np-read-track { flex:1; height:3px; border-radius:3px; background:rgba(255,255,255,.1); overflow:hidden; }
.np-read-fill { height:3px; border-radius:3px; }
.np-read-value { width:17px; text-align:right; font-size:8px; color:#8d7b95; }
.np-body { padding-bottom:12px; }
.np-art-wrap { max-width:400px; }
.grain-layer { position:fixed; left:0; top:0; width:100vw; height:100vh; z-index:75;
  pointer-events:none; opacity:.009; background-repeat:repeat; background-size:256px 256px; }
.seek-hit { flex:1; height:20px; display:flex; align-items:center; cursor:pointer; }
.seek-hit .seek { width:100%; flex:0 0 auto; }
.wave-seek { flex:1; height:22px; display:flex; align-items:center; gap:2px; cursor:pointer; }
.wave-bar { flex:1; min-width:1px; border-radius:2px; }
.wave-bar:hover { opacity:.72; }
.volume-bars { width:62px; height:20px; display:flex; align-items:center; gap:2px; cursor:pointer; }
.volume-seg { flex:1; height:11px; border-radius:2px; background:rgba(255,255,255,.13); }
.volume-seg.on { background:var(--cream); }
.volume-seg:hover { background:var(--pink); }
.volume-demo .volume-bars { flex:1; width:auto; height:22px; }
.seek-row .seek, .np-seek .seek { overflow:visible; }

/* Toast / error banner */
.error-banner { display:flex; align-items:center; gap:10px; margin:10px 30px 0; padding:8px 11px;
  border:1px solid rgba(255,107,107,.3); background:rgba(255,107,107,.075); border-radius:8px; }
.error-tag { flex:0 0 auto; font-size:7px; letter-spacing:1.6px; color:#ff8b8b; text-transform:uppercase; }
.error-text { flex:1; min-width:0; font-size:9px; line-height:1.35; color:#d9aeb5; }
.error-dismiss { flex:0 0 auto; font-size:7px; letter-spacing:1.3px; color:#ffb3b3; cursor:pointer; }
.error-dismiss:hover { color:#fff3e4; }

/* Queue drawer */
.queue-scrim { position:fixed; inset:0; z-index:90; display:flex; justify-content:flex-end;
  background:rgba(5,2,8,.58); }
.queue-drawer { width:396px; height:100vh; display:flex; flex-direction:column; padding:22px 20px 14px;
  background:linear-gradient(180deg,#170d1d,#0e0712); border-left:1px solid rgba(255,255,255,.14);
  box-shadow:-26px 0 70px rgba(0,0,0,.5); }
.qd-head { display:flex; align-items:flex-start; justify-content:space-between; gap:12px; }
.qd-kicker { font-size:7px; letter-spacing:2.2px; color:var(--pink); text-transform:uppercase; }
.qd-title-big { margin-top:4px; font-family:var(--display); font-size:24px; font-weight:900; letter-spacing:-.8px; }
.qd-close { width:28px; height:28px; border-radius:50%; border:1px solid var(--line); display:flex;
  align-items:center; justify-content:center; cursor:pointer; }
.qd-close:hover { border-color:var(--pink); }
.qd-count { margin-top:8px; font-size:7px; letter-spacing:1.3px; color:#8d7b95; text-transform:uppercase; }
.qd-list { flex:1; min-height:0; overflow-y:auto; margin-top:10px; }
.qd-row { display:flex; align-items:center; gap:9px; padding:8px 4px; border-bottom:1px solid rgba(255,255,255,.06);
  cursor:pointer; border-radius:6px; }
.qd-row:hover { background:rgba(255,255,255,.045); }
.qd-index { width:18px; flex:0 0 18px; font-size:8px; color:#6f5e76; }
.qd-art { width:40px; height:40px; flex:0 0 40px; border-radius:5px; overflow:hidden; position:relative; }
.qd-art img { width:100%; height:100%; object-fit:cover; }
.qd-info { min-width:0; flex:1; }
.qd-title { font-size:10px; font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.qd-artist { margin-top:2px; font-size:8px; color:#9c89a3; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.qd-reason { margin-top:3px; font-family:var(--serif); font-style:italic; font-size:11px; color:#c69fba;
  white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.qd-dur { font-size:8px; color:#6f5e76; }
.qd-foot { padding-top:10px; border-top:1px solid var(--line); font-family:var(--serif); font-style:italic;
  font-size:11px; color:#8d7b95; }

@media (max-width: 1320px) {
  .rail { width:232px; flex-basis:232px; }
  .topbar { padding:0 22px; }
  .content { padding:16px 22px 24px; }
  .hero { padding:22px 26px 20px; }
  .hero-title { font-size:40px; }
  .hero-serif { font-size:64px; }
  .lede { max-width:360px; font-size:11px; }
  .hero-meta { font-size:7px; }
  .station { height:190px; }
  .station-artword { font-size:22px; letter-spacing:-.8px; }
  .station-info { font-size:8px; right:45px; }
  .station-play { width:28px; height:28px; }
  .section-head { margin:14px 0 8px; }
  .player-track { width:210px; flex-basis:210px; }
  .player-right { width:214px; flex-basis:214px; }
  .search-trigger { max-width:380px; }
  .screen-title { font-size:29px; }
}
@media (max-width: 1160px) {
  .rail { width:214px; flex-basis:214px; padding:20px 12px 14px; }
  .brand img:last-child { width:148px; height:37px; }
  .hero-copy { width:62%; }
  .hero-title { font-size:36px; }
  .hero-serif { font-size:56px; }
  .hero-art .vinyl { width:228px; height:228px; right:4px; }
  .hero-art .cover-card { width:188px; height:188px; right:82px; top:42px; }
  .station-artword { font-size:19px; }
  .station { height:182px; }
  .player-track { width:180px; flex-basis:180px; }
  .player-right { width:190px; flex-basis:190px; }
  .search-trigger { max-width:280px; }
}
/* The vinyl stays static: animated conic-gradient layers band in BUI's effect
   compositor. Motion lives in the equalizers, radar canvas and hover states. */
"#;
