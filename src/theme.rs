//! GAYDAR FM visual system: colors, type and the full BUI stylesheet.

use std::path::PathBuf;

pub const INK: u32 = 0x0b050d;
pub const INK_2: u32 = 0x140a18;
pub const PANEL: u32 = 0x180d1e;
pub const CREAM: u32 = 0xfff3e4;
pub const MUTED: u32 = 0xb9a7bd;
pub const PINK: u32 = 0xff4fa3;
pub const HOT: u32 = 0xff2d87;
pub const LILAC: u32 = 0xc3a6ff;
pub const LIME: u32 = 0xd6ff3f;
pub const CYAN: u32 = 0x7ef9ff;
pub const GOLD: u32 = 0xffcf5c;

pub fn assets_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("GAYDAR_ASSETS_DIR") {
        let dir = PathBuf::from(dir);
        if dir.is_dir() {
            return dir;
        }
    }
    let local = PathBuf::from("assets");
    if local.is_dir() {
        return local;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(base) = exe.parent() {
            let beside = base.join("assets");
            if beside.is_dir() {
                return beside;
            }
            if let Some(prefix) = base.parent() {
                let share = prefix.join("share").join("gaydar").join("assets");
                if share.is_dir() {
                    return share;
                }
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

pub fn asset(rel: &str) -> String {
    assets_dir().join(rel).to_string_lossy().into_owned()
}

pub fn icon_path(name: &str) -> String {
    asset(&format!("icons/{name}.svg"))
}

/// Register the bundled display faces if the asset agent has put them in place.
/// Missing files silently fall back to Inter, so the app still runs from source.
pub fn load_fonts() {
    let fonts = [
        ("Unbounded", true, false, "unbounded/Unbounded-Black.ttf"),
        ("Unbounded", true, false, "unbounded/Unbounded-Bold.ttf"),
        (
            "Instrument Serif",
            false,
            true,
            "instrument-serif/InstrumentSerif-Italic.ttf",
        ),
        ("DM Sans", false, false, "dm-sans/DMSans-Regular.ttf"),
        ("DM Sans", true, false, "dm-sans/DMSans-Bold.ttf"),
        ("DM Sans", true, false, "dm-sans/DMSans-Medium.ttf"),
    ];
    for (family, bold, italic, file) in fonts {
        let path = assets_dir().join("fonts").join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            blenderui::add_font(family, bold, italic, &bytes);
        }
    }
}

pub const CSS: &str = r#"
:root {
  --ink:#0b050d; --ink2:#140a18; --panel:#180d1e; --panel2:#211028;
  --cream:#fff3e4; --muted:#b9a7bd; --pink:#ff4fa3; --hot:#ff2d87;
  --lilac:#c3a6ff; --lime:#d6ff3f; --cyan:#7ef9ff; --gold:#ffcf5c;
  --line:rgba(255,255,255,.10); --line-strong:rgba(255,255,255,.18);
  --display:'Unbounded','DejaVu Sans',sans-serif;
  --serif:'Instrument Serif','DejaVu Serif',serif;
  --body:'DM Sans',Inter,sans-serif;
  --shadow: 0 22px 60px rgba(0,0,0,.42);
}
* { box-sizing: border-box; }
html, body { margin:0; padding:0; height:100%; background:#0b050d; }
body { font-family:var(--body); color:var(--cream); -webkit-font-smoothing:antialiased; }
button, input, select, textarea { font-family:var(--body); }
.app { height:100vh; width:100vw; display:flex; flex-direction:column; overflow:hidden;
  background:
    radial-gradient(circle at 82% 4%, rgba(255,79,163,.13), transparent 34%),
    radial-gradient(circle at 12% 92%, rgba(117,74,255,.12), transparent 38%),
    #0b050d; }
.pride-bar { height:6px; flex:0 0 6px; width:100%;
  background:linear-gradient(90deg,#ff4fa3 0 16%,#ff9d5c 16% 33%,#ffd166 33% 50%,#4ade80 50% 66%,#73f7ff 66% 83%,#b899ff 83% 100%); }
.shell { flex:1; min-height:0; display:flex; }
.rail { width:248px; flex:0 0 248px; min-height:0; display:flex; flex-direction:column;
  padding:24px 16px 16px; border-right:1px solid var(--line);
  background:linear-gradient(180deg, rgba(255,255,255,.022), transparent 30%), #0d0711; }
.brand { display:flex; align-items:center; gap:11px; padding:2px 8px 0; }
.logo-mark { width:44px; height:44px; border-radius:50%; position:relative; overflow:hidden;
  background:conic-gradient(from 210deg,#ff4fa3,#ffcf5c,#d6ff3f,#7ef9ff,#c3a6ff,#ff4fa3);
  border:1px solid rgba(255,255,255,.35); flex:0 0 44px; }
.logo-mark:after { content:""; position:absolute; inset:7px; border-radius:50%; background:#120817;
  border:1px solid rgba(255,255,255,.18); }
.logo-heart { position:absolute; inset:0; z-index:2; display:flex; align-items:center; justify-content:center;
  color:#ff4fa3; font-size:20px; line-height:1; text-shadow:0 0 12px rgba(255,79,163,.9); }
.brand-word { display:flex; flex-direction:column; gap:2px; }
.brand-name { font-family:var(--display); font-weight:900; font-size:20px; letter-spacing:-.4px; line-height:1; }
.brand-sub { font-size:8px; letter-spacing:3.1px; color:var(--muted); text-transform:uppercase; }
.guest-card { margin:22px 6px 18px; padding:13px 13px 12px; border:1px solid rgba(255,79,163,.35);
  border-radius:14px; background:linear-gradient(135deg, rgba(255,79,163,.14), rgba(195,166,255,.06));
  position:relative; overflow:hidden; }
.guest-card:after { content:""; position:absolute; width:80px; height:80px; border-radius:50%;
  right:-28px; top:-30px; background:radial-gradient(circle, rgba(255,207,92,.35), transparent 68%); }
.guest-kicker { font-size:8px; letter-spacing:2px; color:var(--pink); text-transform:uppercase; }
.guest-title { font-family:var(--serif); font-style:italic; font-size:24px; line-height:1; margin-top:4px; }
.guest-note { margin-top:5px; font-size:10px; color:var(--muted); line-height:1.45; }
.nav-label { padding:0 10px; margin:4px 0 6px; font-size:8px; letter-spacing:2.2px; color:#796a80; text-transform:uppercase; }
.nav { display:flex; flex-direction:column; gap:3px; }
.nav-item { height:40px; display:flex; align-items:center; gap:11px; padding:0 12px; border-radius:11px;
  color:#cbbcd0; font-size:12.5px; font-weight:500; cursor:pointer; border:1px solid transparent;
  transition: background .16s ease, color .16s ease, transform .16s ease; }
.nav-item:hover { background:rgba(255,255,255,.055); color:var(--cream); transform:translateX(2px); }
.nav-item.active { background:var(--pink); color:#160812; border-color:rgba(255,255,255,.28);
  box-shadow:0 7px 22px rgba(255,79,163,.22); font-weight:700; }
.nav-item.active .nav-icon { background:#160812; }
.nav-icon { width:16px; height:16px; flex:0 0 16px; background:#b9a7bd; opacity:.92;
  -webkit-mask-size:contain; mask-size:contain; -webkit-mask-repeat:no-repeat; mask-repeat:no-repeat;
  -webkit-mask-position:center; mask-position:center; }
.nav-item.active .nav-icon { background:#160812; opacity:1; }
.nav-kbd { margin-left:auto; font-size:8px; letter-spacing:.6px; color:#7c6d82; }
.nav-item.active .nav-kbd { color:rgba(22,8,18,.55); }
.rail-spacer { flex:1; min-height:14px; }
.rail-mini { margin:0 4px 10px; padding:11px 11px; border:1px solid var(--line); border-radius:12px;
  background:rgba(255,255,255,.025); }
.rail-mini-label { font-size:8px; letter-spacing:1.8px; color:#827188; text-transform:uppercase; }
.rail-mini-row { display:flex; align-items:center; gap:8px; margin-top:6px; }
.alsa-dot { width:6px; height:6px; border-radius:50%; background:var(--lime); box-shadow:0 0 8px rgba(214,255,63,.65); }
.alsa-name { font-size:10px; color:#d7c8db; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.alsa-meta { margin-top:2px; font-size:8px; color:#7c6d82; }
.rail-foot { padding:10px 8px 2px; font-size:8px; color:#6f6075; line-height:1.6; letter-spacing:.2px; }
.heart-rainbow { letter-spacing:1px; }
.main { flex:1; min-width:0; min-height:0; display:flex; flex-direction:column; }
.topbar { flex:0 0 66px; height:66px; display:flex; align-items:center; gap:18px; padding:0 30px;
  border-bottom:1px solid var(--line); background:rgba(11,5,13,.5); }
.topbar-context { display:flex; flex-direction:column; gap:3px; min-width:190px; }
.topbar-eyebrow { font-size:8px; letter-spacing:2.3px; color:#8c7b92; text-transform:uppercase; }
.topbar-screen { font-size:12px; font-weight:600; letter-spacing:.2px; }
.search-trigger { flex:1; max-width:430px; height:36px; display:flex; align-items:center; gap:10px;
  padding:0 13px; border-radius:10px; border:1px solid var(--line); background:rgba(255,255,255,.035);
  color:#8d7d94; font-size:11px; cursor:pointer; transition:border-color .16s ease, background .16s ease; }
.search-trigger:hover { border-color:rgba(255,79,163,.45); background:rgba(255,79,163,.06); color:#d9cadf; }
.search-trigger-icon { width:14px; height:14px; border:1.5px solid currentColor; border-radius:50%;
  position:relative; opacity:.8; }
.search-trigger-icon:after { content:""; position:absolute; width:5px; height:1.5px; background:currentColor;
  transform:rotate(45deg); right:-4px; bottom:0; }
.search-key { margin-left:auto; padding:2px 6px; border:1px solid var(--line); border-radius:5px;
  font-size:8px; color:#8c7b92; }
.topbar-right { margin-left:auto; display:flex; align-items:center; gap:14px; }
.live-chip { display:flex; align-items:center; gap:7px; height:28px; padding:0 10px; border-radius:7px;
  border:1px solid rgba(214,255,63,.26); background:rgba(214,255,63,.07); color:var(--lime);
  font-size:8px; letter-spacing:1.8px; text-transform:uppercase; }
.live-dot { width:5px; height:5px; border-radius:50%; background:var(--lime); box-shadow:0 0 9px var(--lime); }
.icon-btn { width:32px; height:32px; border-radius:9px; border:1px solid var(--line);
  background:rgba(255,255,255,.035); display:flex; align-items:center; justify-content:center; cursor:pointer;
  color:#cbbcd0; transition:background .15s ease, border-color .15s ease; }
.icon-btn:hover { background:rgba(255,79,163,.14); border-color:rgba(255,79,163,.45); }
.gear { width:15px; height:15px; border:1.5px solid currentColor; border-radius:50%; position:relative; opacity:.8; }
.gear:after { content:""; position:absolute; inset:4px; border-radius:50%; background:currentColor; }
.content { flex:1; min-height:0; overflow-y:auto; padding:18px 30px 28px; }
.content-wide { padding-top:18px; }
.section-head { display:flex; align-items:flex-end; justify-content:space-between; margin:16px 0 8px; }
.section-title { font-family:var(--display); font-size:13px; font-weight:800; letter-spacing:.2px; }
.section-sub { margin-left:10px; font-size:9px; letter-spacing:1.7px; color:#83728a; text-transform:uppercase; }
.section-link { font-size:9px; letter-spacing:1.3px; color:#a693ae; text-transform:uppercase; cursor:pointer; }
.section-link:hover { color:var(--pink); }
.hero { position:relative; overflow:hidden; min-height:298px; display:flex; border-radius:26px;
  padding:26px 32px 24px; border:1px solid rgba(255,255,255,.13);
  background:
    radial-gradient(circle at 84% 18%, rgba(255,207,92,.20), transparent 24%),
    radial-gradient(circle at 72% 82%, rgba(126,249,255,.11), transparent 30%),
    linear-gradient(125deg, #2a0d2b 0%, #3b1238 38%, #58153f 70%, #2b0e33 100%);
  box-shadow: var(--shadow); }
/* Deliberately no animated/filtered pseudo-element here: BUI's effect layer
   bands conic gradients inside animated layers. The hero gets its depth from
   the layered background and the vinyl/card composition instead. */
.hero:after { content:""; position:absolute; left:36px; right:36px; bottom:0; height:1px;
  background:linear-gradient(90deg, transparent, rgba(255,255,255,.22), transparent); }
.hero-copy { position:relative; z-index:2; width:56%; padding:6px 0 0 2px; display:flex; flex-direction:column; }
.kicker { display:flex; align-items:center; gap:9px; font-size:8px; letter-spacing:2.5px; color:#f6b8d4;
  text-transform:uppercase; }
.kicker-line { width:22px; height:1px; background:#f6b8d4; opacity:.7; }
.hero-title { margin-top:10px; font-family:var(--display); font-weight:900; font-size:43px; line-height:1.02;
  letter-spacing:-1.7px; color:var(--cream); text-shadow:0 2px 30px rgba(0,0,0,.22); }
.hero-serif { display:block; font-family:var(--serif); font-style:italic; font-weight:400;
  font-size:74px; line-height:.82; letter-spacing:-1px; color:#ffe6f1; margin-top:1px; }
.lede { margin-top:10px; max-width:405px; color:#d7bfd0; font-size:12px; line-height:1.55; }
.hero-actions { display:flex; align-items:center; gap:10px; margin-top:16px; }
.btn-primary { display:flex; align-items:center; gap:8px; height:40px; padding:0 19px; border-radius:9px; border:1px solid rgba(255,255,255,.3);
  background:var(--pink); color:#170912; font-family:var(--body); font-size:10px; font-weight:800;
  letter-spacing:1.5px; text-transform:uppercase; cursor:pointer; box-shadow:0 8px 24px rgba(255,79,163,.30);
  transition:transform .16s ease, background .16s ease; }
.btn-primary:hover { background:#ff6bb5; transform:translateY(-1px); }
.btn-ghost { display:flex; align-items:center; gap:8px; height:40px; padding:0 17px; border-radius:9px; border:1px solid rgba(255,255,255,.28);
  background:rgba(255,255,255,.05); color:var(--cream); font-size:9px; font-weight:700;
  letter-spacing:1.5px; text-transform:uppercase; cursor:pointer; transition:background .16s ease; }
.btn-ghost:hover { background:rgba(255,255,255,.12); }
.hero-meta { margin-top:auto; padding-top:12px; display:flex; align-items:center; gap:13px;
  color:#b990ab; font-size:8px; letter-spacing:1.3px; text-transform:uppercase; }
.hero-meta-sep { width:3px; height:3px; border-radius:50%; background:#b990ab; }
.hero-art { position:relative; flex:1; min-width:0; z-index:2; }
.vinyl { position:absolute; width:264px; height:264px; right:18px; top:14px; border-radius:50%;
  background:conic-gradient(from 45deg,#17101b,#2b1731 8%,#17101b 16%,#25142b 24%,#17101b 32%,#2b1731 40%,#17101b 48%,#25142b 56%,#17101b 64%,#2b1731 72%,#17101b 80%,#25142b 88%,#17101b);
  box-shadow:0 20px 55px rgba(0,0,0,.45), inset 0 0 0 1px rgba(255,255,255,.10); }
.vinyl:after { content:""; position:absolute; inset:78px; border-radius:50%;
  background:radial-gradient(circle, #ffcf5c 0 18%, #ff4fa3 19% 48%, #24102a 49% 100%);
  border:1px solid rgba(255,255,255,.25); }
.cover-card { position:absolute; width:216px; height:216px; right:104px; top:44px; border-radius:5px;
  transform:rotate(-3deg); overflow:hidden; border:1px solid rgba(255,255,255,.35);
  box-shadow:0 24px 50px rgba(0,0,0,.46); }
.cover-card .cover-art { position:absolute; inset:0; background:
  radial-gradient(circle at 30% 22%, #ffcf5c 0 7%, transparent 8%),
  radial-gradient(circle at 68% 64%, #7ef9ff 0 13%, transparent 14%),
  linear-gradient(145deg,#ff4fa3 0%,#a54dff 44%,#2d0d3a 100%); }
.cover-card .cover-type { position:absolute; left:14px; bottom:11px; right:12px; color:#fff3e4;
  font-family:var(--display); font-weight:900; font-size:22px; line-height:.98; letter-spacing:-.7px;
  text-shadow:0 2px 14px rgba(0,0,0,.4); }
.cover-card .cover-artist { position:absolute; left:15px; top:13px; font-size:8px; letter-spacing:1.8px;
  text-transform:uppercase; color:rgba(255,255,255,.86); }
.cover-card:after { content:""; position:absolute; inset:0; background:linear-gradient(155deg, rgba(255,255,255,.22), transparent 38%); }
.onair { position:absolute; right:14px; top:12px; display:flex; align-items:center; gap:6px;
  padding:5px 8px; border-radius:4px; background:var(--lime); color:#111408; font-size:7px;
  letter-spacing:1.7px; font-weight:900; transform:rotate(5deg); box-shadow:0 6px 18px rgba(0,0,0,.3); }
.onair-dot { width:4px; height:4px; border-radius:50%; background:#111408; }
.sticker { position:absolute; left:2px; bottom:20px; transform:rotate(-7deg); padding:7px 10px;
  background:var(--cream); color:#1b0812; font-family:var(--display); font-size:9px; font-weight:900;
  letter-spacing:.3px; border:1px solid #1b0812; box-shadow:4px 4px 0 rgba(27,8,18,.55); }
.sticker small { display:block; font-family:var(--body); font-size:6px; letter-spacing:1.7px; margin-top:2px; }
.hero-eq { position:relative; left:auto; bottom:auto; display:flex; align-items:flex-end; gap:3px; height:15px; margin-right:5px; }
.hero-eq span { width:3px; border-radius:2px; background:#ffcf5c; transform-origin:bottom; animation:eq 1.1s ease-in-out infinite; }
.hero-eq span:nth-child(2) { animation-delay:-.3s; background:#ff4fa3; }
.hero-eq span:nth-child(3) { animation-delay:-.6s; background:#c3a6ff; }
.hero-eq span:nth-child(4) { animation-delay:-.15s; background:#7ef9ff; }
.hero-eq span:nth-child(5) { animation-delay:-.45s; background:#d6ff3f; }
.station-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:14px; }
.station { position:relative; height:200px; border-radius:15px; overflow:hidden; cursor:pointer;
  border:1px solid rgba(255,255,255,.16); box-shadow:0 12px 28px rgba(0,0,0,.24);
  transition:transform .18s ease, box-shadow .18s ease; }
.station:hover { transform:translateY(-4px) rotate(-.4deg); box-shadow:0 20px 38px rgba(0,0,0,.36); }
.station-inner { position:absolute; inset:0; padding:15px; display:flex; flex-direction:column; }
.station:after { content:""; position:absolute; inset:0; background:linear-gradient(180deg, transparent 18%, rgba(9,4,12,.82) 100%); }
.station.camp { background:radial-gradient(circle at 72% 24%, #ffd166 0 7%, transparent 8%), radial-gradient(circle at 25% 34%, rgba(255,255,255,.22), transparent 24%), linear-gradient(150deg,#ff2d87 0%,#ff7c4f 54%,#5d1135 100%); }
.station.ballroom { background:radial-gradient(circle at 66% 27%, #7ef9ff 0 4%, transparent 5%), repeating-radial-gradient(circle at 64% 30%, transparent 0 17px, rgba(255,255,255,.08) 18px 19px), linear-gradient(145deg,#0d0711,#2b123d 56%,#7d1f72); }
.station.soft { background:radial-gradient(circle at 30% 26%, rgba(255,255,255,.55) 0 6%, transparent 7%), linear-gradient(165deg,#ffd7d1,#d9a6ff 52%,#7f74d6); }
.station.hyper { background:repeating-linear-gradient(90deg, rgba(255,255,255,.10) 0 1px, transparent 1px 13px), linear-gradient(155deg,#d6ff3f,#4ee7d4 45%,#6d45ff 100%); }
.station-top { display:flex; justify-content:space-between; align-items:flex-start; position:relative; z-index:2; }
.station-tag { padding:3px 6px; background:rgba(12,5,14,.34); border:1px solid rgba(255,255,255,.24);
  border-radius:3px; font-size:7px; letter-spacing:1.4px; text-transform:uppercase; color:#fff5e9; }
.station-num { font-family:var(--serif); font-style:italic; font-size:18px; color:rgba(255,255,255,.7); }
.station-artword { position:relative; z-index:2; margin-top:18px; font-family:var(--display); font-weight:900;
  font-size:28px; letter-spacing:-1.2px; line-height:.9; text-transform:uppercase; color:#fff6e9;
  text-shadow:0 2px 20px rgba(0,0,0,.25); }
.station.soft .station-artword { color:#2d1538; text-shadow:none; }
.station.soft .station-tag { background:rgba(255,255,255,.32); color:#2d1538; border-color:rgba(45,21,56,.3); }
.station.soft .station-info { color:#3c2149; }
.station.soft .station-play { background:#fff3e4; color:#2d1538; }
.station-info { position:absolute; left:15px; right:56px; bottom:14px; z-index:2; color:#e9d8e9;
  font-size:9px; line-height:1.35; letter-spacing:.15px; }
.station-play { position:absolute; right:13px; bottom:13px; z-index:3; width:32px; height:32px;
  border-radius:50%; background:#fff3e4; color:#1a0914; display:flex; align-items:center; justify-content:center;
  font-size:10px; box-shadow:0 6px 16px rgba(0,0,0,.3); transition:transform .15s ease; }
.station:hover .station-play { transform:scale(1.12); }
.station-artdeco { position:absolute; right:24px; top:48px; width:92px; height:92px; border-radius:50%;
  border:1px solid rgba(255,255,255,.22); opacity:.6; }
.station-artdeco:after { content:""; position:absolute; inset:17px; border-radius:50%;
  border:1px solid rgba(255,255,255,.2); }
.tracklist { border-top:1px solid var(--line); }
.track-row { display:flex; align-items:center; gap:11px; min-height:58px; padding:7px 8px;
  border-bottom:1px solid rgba(255,255,255,.055); cursor:pointer; border-radius:7px;
  transition:background .14s ease; }
.track-row:hover { background:rgba(255,255,255,.045); }
.track-index { width:18px; flex:0 0 18px; text-align:center; color:#72617a; font-size:9px; }
.track-art { width:42px; height:42px; flex:0 0 42px; border-radius:5px; overflow:hidden; position:relative;
  background:linear-gradient(135deg,#ff4fa3,#6d45ff); border:1px solid rgba(255,255,255,.14); }
.track-art img { width:100%; height:100%; object-fit:cover; }
.track-art-letter { position:absolute; inset:0; display:flex; align-items:center; justify-content:center;
  font-family:var(--display); font-size:16px; color:rgba(255,255,255,.82); }
.track-info { min-width:0; flex:1; display:flex; flex-direction:column; gap:3px; }
.track-title { font-size:11.5px; font-weight:600; color:#f1e4ef; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.track-artist { font-size:9px; color:#9c89a3; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.track-reason { width:190px; flex:0 0 190px; font-family:var(--serif); font-style:italic; font-size:12px; color:#c69fba; }
.track-dur { width:36px; text-align:right; font-size:9px; color:#7e6d86; }
.track-heart { width:28px; height:28px; display:flex; align-items:center; justify-content:center;
  color:#806d88; font-size:12px; border-radius:50%; cursor:pointer; }
.track-heart:hover { color:var(--pink); background:rgba(255,79,163,.09); }
.track-heart.on { color:var(--pink); }
.player { flex:0 0 100px; height:100px; border-top:1px solid rgba(255,255,255,.12);
  background:rgba(12,6,15,.92); display:flex; align-items:center; gap:22px; padding:0 23px; position:relative; }
.player:before { content:""; position:absolute; left:0; right:0; top:-1px; height:2px;
  background:linear-gradient(90deg,#ff4fa3,#ffd166,#d6ff3f,#7ef9ff,#b899ff); opacity:.85; }
.player-track { display:flex; align-items:center; gap:11px; width:242px; flex:0 0 242px; min-width:0; }
.player-art { width:62px; height:62px; flex:0 0 62px; border-radius:7px; overflow:hidden; position:relative;
  border:1px solid rgba(255,255,255,.18); background:linear-gradient(145deg,#ff4fa3,#5c2a8d); }
.player-art img { width:100%; height:100%; object-fit:cover; }
.player-meta { min-width:0; display:flex; flex-direction:column; gap:4px; }
.player-title { font-size:11px; font-weight:700; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.player-artist { font-size:9px; color:#a28fa9; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.player-heart { margin-left:4px; width:26px; height:26px; border-radius:50%; display:flex; align-items:center;
  justify-content:center; color:#8f7a97; cursor:pointer; font-size:12px; flex:0 0 26px; }
.player-heart:hover { color:var(--pink); background:rgba(255,79,163,.1); }
.player-heart.on { color:var(--pink); }
.player-center { flex:1; min-width:0; display:flex; flex-direction:column; gap:7px; align-items:center; }
.transport { display:flex; align-items:center; gap:12px; }
.small-control { width:24px; height:24px; display:flex; align-items:center; justify-content:center;
  color:#a08ba8; font-size:11px; cursor:pointer; border-radius:50%; }
.small-control:hover { color:var(--cream); background:rgba(255,255,255,.07); }
.small-control.active { color:var(--lime); }
.play-btn { width:38px; height:38px; border-radius:50%; background:var(--pink); color:#1b0912;
  display:flex; align-items:center; justify-content:center; cursor:pointer; border:1px solid rgba(255,255,255,.35);
  box-shadow:0 6px 20px rgba(255,79,163,.32); font-size:13px; font-weight:800; padding-left:2px;
  transition:transform .15s ease, background .15s ease; }
.play-btn:hover { transform:scale(1.06); background:#ff68b2; }
.play-btn.paused { padding-left:0; }
.seek-row { width:100%; max-width:540px; display:flex; align-items:center; gap:9px; }
.time { width:31px; font-size:8px; color:#856f8d; letter-spacing:.3px; }
.time.right { text-align:right; }
.seek { flex:1; height:6px; border-radius:5px; background:rgba(255,255,255,.11); cursor:pointer; position:relative; }
.seek-fill { height:6px; border-radius:5px; background:linear-gradient(90deg,#ff4fa3,#ffcf5c 55%,#7ef9ff); }
.seek-thumb { position:absolute; width:9px; height:9px; border-radius:50%; background:#fff3e4; top:-2px;
  box-shadow:0 0 0 2px rgba(255,79,163,.4); }
.player-right { width:246px; flex:0 0 246px; display:flex; align-items:center; justify-content:flex-end; gap:13px; }
.mini-viz { display:flex; align-items:flex-end; gap:2px; height:22px; width:66px; }
.mini-viz span { width:2px; border-radius:2px; background:#c3a6ff; opacity:.8; }
.volume-wrap { display:flex; align-items:center; gap:7px; }
.volume-icon { width:13px; height:13px; border:1.4px solid #a08ba8; border-radius:2px 5px 5px 2px; position:relative; }
.volume-icon:before { content:""; position:absolute; left:-5px; top:3px; width:5px; height:5px; background:#a08ba8; }
.volume-track { width:62px; height:4px; background:rgba(255,255,255,.12); border-radius:4px; cursor:pointer; }
.volume-fill { height:4px; background:var(--cream); border-radius:4px; }
.queue-btn { width:28px; height:28px; border-radius:7px; border:1px solid var(--line); display:flex;
  align-items:center; justify-content:center; color:#a08ba8; cursor:pointer; font-size:10px; }
.queue-btn:hover { border-color:var(--pink); color:var(--pink); }
.queue-count { position:absolute; margin-top:-22px; margin-left:15px; background:var(--pink); color:#1b0912;
  font-size:7px; font-weight:800; width:13px; height:13px; border-radius:50%; display:flex; align-items:center; justify-content:center; }
.empty-state { display:flex; flex-direction:column; align-items:center; justify-content:center; height:100%;
  padding:38px; text-align:center; }
.empty-title { font-family:var(--display); font-size:16px; font-weight:800; }
.empty-note { margin-top:8px; max-width:300px; font-size:11px; color:#9c89a3; line-height:1.55; }
.toast { position:fixed; left:50%; bottom:114px; transform:translateX(-50%); z-index:80;
  padding:9px 14px; border-radius:8px; border:1px solid rgba(255,255,255,.2); background:#221028;
  color:#f7e9f4; font-size:10px; box-shadow:0 10px 30px rgba(0,0,0,.4); display:flex; align-items:center; gap:8px; }
.toast-dot { width:5px; height:5px; border-radius:50%; background:var(--pink); }
.toast.good .toast-dot { background:var(--lime); }
.toast.bad .toast-dot { background:#ff7b7b; }
@keyframes spin { from { transform:rotate(0deg); } to { transform:rotate(360deg); } }
@keyframes eq { 0%,100% { transform:scaleY(.35); } 50% { transform:scaleY(1); } }
@keyframes floaty { 0%,100% { transform:translateY(0) rotate(-2deg); } 50% { transform:translateY(-8px) rotate(1deg); } }
@keyframes sweep { from { transform:rotate(0deg); } to { transform:rotate(360deg); } }
@keyframes shimmer { 0% { opacity:.35; } 50% { opacity:.9; } 100% { opacity:.35; } }
"#;
