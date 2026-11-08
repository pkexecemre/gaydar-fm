//! GAYDAR FM — "Now Playing" visual prototype for blenderui 0.5.
//!
//! A disposable, self-contained mock of the production Now Playing screen.
//! No network, no audio, no auth: all data below is fictional except the
//! current track. Run it natively, or render the 1440x900 frame headlessly:
//!
//! ```text
//! cargo run --release --example now_playing
//! cargo run --release --example now_playing -- --screenshot out.png
//! cargo run --release --example now_playing -- --screenshot out.png --at 2600
//! ```
//!
//! Type: IBM Plex Sans Condensed / Serif Italic / Mono via `@font-face` from
//! the system font directory. Production should bundle those faces instead
//! (see docs/BUI-NOTES.md); BUI falls back to its embedded Inter if absent.

use std::time::Duration;

use blenderui::prelude::*;

const CSS: &str = r###"
@font-face { font-family: "Gaydar Display"; src: url("/usr/share/fonts/truetype/IBMPlexSansCondensed-Bold.otf"); font-weight: 700; }
@font-face { font-family: "Gaydar Display"; src: url("/usr/share/fonts/truetype/IBMPlexSansCondensed-SemiBold.otf"); font-weight: 600; }
@font-face { font-family: "Gaydar Serif"; src: url("/usr/share/fonts/truetype/IBMPlexSerif-LightItalic.otf"); font-style: italic; font-weight: 300; }
@font-face { font-family: "Gaydar Serif"; src: url("/usr/share/fonts/truetype/IBMPlexSerif-Italic.otf"); font-style: italic; font-weight: 400; }

:root {
  --bg: #09050d; --ink: #f8efe8; --muted: #a28eae; --dim: #76637f;
  --pink: #ff4fa3; --pink2: #ff8ac1; --deep: #d62b7a; --lilac: #c7a6ff; --cyan: #59e6ff; --lime: #c9ff5c; --cream: #fff3e9;
  --line: rgba(255, 243, 233, 0.10); --line2: rgba(255, 243, 233, 0.055);
}
* { box-sizing: border-box; }
html, body { margin: 0; }
body { background: var(--bg); color: var(--ink); font-family: sans-serif; }
.app { width: 1440px; height: 900px; position: relative; overflow: hidden;
  background: linear-gradient(180deg, #0b0611 0%, #09050d 52%, #0c0712 100%); }
.shell { position: absolute; left: 0; top: 0; width: 1440px; height: 900px; display: flex; z-index: 2; }
.bg-grain { position: absolute; left: 0; top: 0; width: 1440px; height: 900px; z-index: 1; }
.heart-clip { clip-path: polygon(50.00% 23.71%, 50.10% 22.58%, 50.76% 19.40%, 52.47% 14.81%, 55.50% 9.72%, 59.93% 5.15%, 65.56% 2.01%, 71.97% 0.97%, 78.58% 2.36%, 84.70% 6.11%, 89.65% 11.86%, 92.88% 19.03%, 94.00% 27.00%, 92.88% 35.21%, 89.65% 43.26%, 84.70% 50.94%, 78.58% 58.21%, 71.97% 65.12%, 65.56% 71.71%, 59.93% 77.95%, 55.50% 83.71%, 52.47% 88.71%, 50.76% 92.63%, 50.10% 95.13%, 50.00% 96.00%, 49.90% 95.13%, 49.24% 92.63%, 47.53% 88.71%, 44.50% 83.71%, 40.07% 77.95%, 34.44% 71.71%, 28.03% 65.12%, 21.42% 58.21%, 15.30% 50.94%, 10.35% 43.26%, 7.12% 35.21%, 6.00% 27.00%, 7.12% 19.03%, 10.35% 11.86%, 15.30% 6.11%, 21.42% 2.36%, 28.03% 0.97%, 34.44% 2.01%, 40.07% 5.15%, 44.50% 9.72%, 47.53% 14.81%, 49.24% 19.40%, 49.90% 22.58%); }

.pride-edge { position: absolute; left: 0; top: 0; width: 1440px; height: 3px; z-index: 40;
  background: linear-gradient(90deg, #ff4fa3 0%, #ff8a5c 18%, #ffe066 34%, #b6ff5c 50%, #59e6ff 68%, #c7a6ff 86%, #ff4fa3 100%); opacity: 0.9; }
.glow { position: absolute; border-radius: 50%; z-index: 0; }
.glow-a { left: 180px; top: 32px; width: 700px; height: 700px;
  background: radial-gradient(circle, rgba(255,79,163,0.10) 0%, rgba(255,79,163,0.03) 40%, rgba(255,79,163,0) 70%); }
.glow-b { left: 820px; top: -160px; width: 680px; height: 680px;
  background: radial-gradient(circle, rgba(199,166,255,0.055) 0%, rgba(199,166,255,0) 68%); }
.glow-c { left: 1000px; top: 560px; width: 560px; height: 560px;
  background: radial-gradient(circle, rgba(89,230,255,0.035) 0%, rgba(89,230,255,0) 68%); }
.glow-d { left: 250px; top: 560px; width: 520px; height: 420px;
  background: radial-gradient(circle, rgba(214,43,122,0.055) 0%, rgba(214,43,122,0) 70%); }
.ghost-wrap { position: absolute; left: 360px; top: 604px; width: 1040px; height: 300px; z-index: 1; overflow: hidden; }
.ghost { position: absolute; font-family: "Gaydar Display", "IBM Plex Sans", sans-serif; font-weight: 700;
  font-size: 226px; line-height: 0.8; letter-spacing: -0.025em; white-space: nowrap; }
.ghost-1 { left: 0; top: 0; color: rgba(255,243,233,0.030); }
.ghost-2 { left: 6px; top: 5px; color: rgba(255,79,163,0.024); }
.big-rays { position: absolute; left: 410px; top: 300px; width: 0; height: 0; z-index: 0; opacity: 0.6; }
.big-rays .ray { position: absolute; left: -1px; bottom: 0; width: 2px; height: 560px; transform-origin: 50% 100%;
  background: linear-gradient(to top, rgba(255,79,163,0.09), rgba(255,79,163,0.02) 55%, rgba(255,79,163,0) 88%); }
.halo { position: absolute; left: 470px; top: 500px; width: 560px; height: 220px; z-index: 0;
  background: radial-gradient(ellipse, rgba(255,79,163,0.05), rgba(255,79,163,0) 70%); }
.orbit-bg { position: absolute; left: 92px; top: 100px; width: 700px; height: 700px; border-radius: 350px;
  border: 1px solid rgba(255,243,233,0.05); z-index: 0; }
.orbit-bg2 { position: absolute; left: 190px; top: 198px; width: 504px; height: 504px; border-radius: 252px;
  border: 1px dashed rgba(255,243,233,0.07); z-index: 0; }

.mono { font-family: "IBM Plex Mono", monospace; font-size: 8px; letter-spacing: 0.20em; text-transform: uppercase; color: var(--muted); }
.mono-pink { color: var(--pink); }
.mono-cream { color: var(--cream); }
.serif { font-family: "Gaydar Serif", "IBM Plex Serif", serif; font-style: italic; font-weight: 300; }
.display { font-family: "Gaydar Display", "IBM Plex Sans", sans-serif; font-weight: 700; }
.dot { display: inline-block; width: 3px; height: 3px; border-radius: 2px; background: var(--pink); margin: 0 7px 2px 7px; }
.hair { height: 1px; background: var(--line); }

.rail-left { width: 228px; height: 900px; flex-shrink: 0; border-right: 1px solid var(--line);
  background: linear-gradient(90deg, rgba(9,5,13,0.94), rgba(9,5,13,0.74));
  padding: 26px 20px 20px 20px; display: flex; flex-direction: column; }
.brand { display: flex; align-items: center; gap: 11px; }
.mark { position: relative; width: 44px; height: 44px; flex-shrink: 0; }
.mark-ring { position: absolute; border: 1px solid rgba(255,79,163,0.36); border-radius: 50%; }
.mark-r1 { left: 0; top: 0; width: 44px; height: 44px; }
.mark-r2 { left: 7px; top: 7px; width: 30px; height: 30px; border-color: rgba(255,79,163,0.24); }
.mark-heart { position: absolute; left: 15px; top: 13px; width: 15px; height: 15px;
  background: linear-gradient(140deg, #ff9ccb, #ff4fa3 55%, #d62b7a); }
.mark-dot { position: absolute; left: 32px; top: 7px; width: 4px; height: 4px; border-radius: 2px; background: var(--lime);
  box-shadow: 0 0 6px rgba(201,255,92,0.8); }
.mark-sweep { position: absolute; left: 22px; top: 22px; width: 22px; height: 1px; transform-origin: 0 50%;
  background: linear-gradient(90deg, rgba(255,79,163,0.8), rgba(255,79,163,0)); animation: sweep 6s linear infinite; }
.wordmark { display: flex; flex-direction: column; gap: 3px; }
.wordmark-main { font-family: "Gaydar Display", "IBM Plex Sans", sans-serif; font-weight: 700; font-size: 19px;
  letter-spacing: 0.17em; color: var(--cream); line-height: 1; }
.wordmark-main .fm { color: var(--pink); }
.wordmark-sub { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.22em; color: var(--muted); }
.station-line { display: flex; align-items: center; gap: 8px; margin-top: 24px; }
.station-line .bar { width: 22px; height: 2px; background: linear-gradient(90deg, var(--pink), var(--lilac)); }
.station-line .txt { font-family: "IBM Plex Mono", monospace; font-size: 7px; letter-spacing: 0.17em; color: var(--dim); }
.nav-label { margin-top: 34px; display: flex; justify-content: space-between; align-items: center; }
.nav-label .mini { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.14em; color: var(--dim); }
.nav { margin-top: 9px; display: flex; flex-direction: column; gap: 1px; }
.nav-item { height: 38px; display: flex; align-items: center; gap: 11px; padding: 0 9px; border-radius: 3px;
  position: relative; cursor: pointer; transition: background 0.2s ease; }
.nav-item:hover { background: rgba(255,243,233,0.045); }
.nav-item .n { font-family: "IBM Plex Mono", monospace; font-size: 7px; color: var(--dim); width: 12px; letter-spacing: 0.05em; }
.nav-item .lbl { font-size: 12px; font-weight: 500; letter-spacing: 0.02em; color: #cbbdce; flex: 1; }
.nav-item.active { background: linear-gradient(90deg, rgba(255,79,163,0.17), rgba(255,79,163,0.015) 82%, transparent); }
.nav-item.active:before { content: ""; position: absolute; left: 0; top: 9px; width: 2px; height: 20px; background: var(--pink); }
.nav-item.active .lbl { color: var(--cream); }
.nav-item.active .n { color: var(--pink); }
.nav-item .live-dot { width: 4px; height: 4px; border-radius: 2px; background: var(--pink); box-shadow: 0 0 6px rgba(255,79,163,0.8); }
.icon { width: 17px; height: 17px; position: relative; flex-shrink: 0; }
.ic-heart { position: absolute; left: 2px; top: 3px; width: 12px; height: 11px; background: currentColor; }
.ic-heart.fill { background: linear-gradient(140deg, var(--pink2), var(--pink)); }
.icon.ic-pink { color: var(--pink); } .icon.ic-muted { color: #93809f; }
.ic-radar { position: absolute; left: 1px; top: 1px; width: 14px; height: 14px; border: 1px solid currentColor; border-radius: 8px; opacity: 0.5; }
.ic-radar2 { position: absolute; left: 5px; top: 5px; width: 6px; height: 6px; border: 1px solid currentColor; border-radius: 4px; }
.ic-radar3 { position: absolute; left: 8px; top: 8px; width: 2px; height: 2px; background: currentColor; border-radius: 1px; }
.ic-search { position: absolute; left: 2px; top: 2px; width: 10px; height: 10px; border: 1.5px solid currentColor; border-radius: 7px; }
.ic-search-h { position: absolute; left: 11px; top: 12px; width: 6px; height: 1.5px; background: currentColor; transform: rotate(45deg); }
.ic-clock { position: absolute; left: 1px; top: 1px; width: 14px; height: 14px; border: 1px solid currentColor; border-radius: 8px; }
.ic-clock-h { position: absolute; left: 8px; top: 4px; width: 1px; height: 5px; background: currentColor; }
.ic-clock-m { position: absolute; left: 8px; top: 8px; width: 4px; height: 1px; background: currentColor; }

.radar-box { margin-top: 31px; padding: 14px 8px 12px 8px; border-top: 1px solid var(--line2); border-bottom: 1px solid var(--line2); }
.radar-wrap { position: relative; width: 146px; height: 146px; margin: 0 auto; border-radius: 73px; overflow: hidden;
  background: radial-gradient(circle, rgba(255,79,163,0.10), rgba(255,79,163,0.015) 58%, transparent 72%); }
.radar-ring { position: absolute; border: 1px solid rgba(255,79,163,0.22); border-radius: 50%; }
.rr1 { left: 10px; top: 10px; width: 126px; height: 126px; }
.rr2 { left: 35px; top: 35px; width: 76px; height: 76px; border-color: rgba(255,79,163,0.16); }
.rr3 { left: 58px; top: 58px; width: 30px; height: 30px; border-color: rgba(255,79,163,0.26); }
.radar-cross-h { position: absolute; left: 10px; top: 72px; width: 126px; height: 1px; background: rgba(255,79,163,0.12); }
.radar-cross-v { position: absolute; left: 72px; top: 10px; width: 1px; height: 126px; background: rgba(255,79,163,0.12); }
.radar-sweep { position: absolute; left: 73px; top: 73px; width: 66px; height: 66px; transform-origin: 0 0;
  background: linear-gradient(45deg, rgba(255,79,163,0.26), rgba(255,79,163,0.03) 55%, transparent 70%);
  border-radius: 0 0 66px 0; animation: sweep 7s linear infinite; }
.radar-arm { position: absolute; left: 73px; top: 73px; width: 64px; height: 1px; transform-origin: 0 50%;
  background: linear-gradient(90deg, rgba(255,79,163,0.9), rgba(255,79,163,0)); animation: sweep 7s linear infinite; }
.blip { position: absolute; width: 5px; height: 5px; border-radius: 3px; background: var(--pink); box-shadow: 0 0 7px rgba(255,79,163,0.9);
  animation: blip 2.8s ease-in-out infinite; }
.blip.a { left: 41px; top: 37px; } .blip.b { left: 101px; top: 62px; animation-delay: -0.9s; background: var(--cyan); box-shadow: 0 0 7px rgba(89,230,255,0.8); }
.blip.c { left: 63px; top: 101px; animation-delay: -1.7s; background: var(--lime); box-shadow: 0 0 7px rgba(201,255,92,0.8); }
.radar-caption { margin-top: 10px; display: flex; align-items: baseline; justify-content: space-between; }
.radar-caption .mono { font-size: 6.5px; letter-spacing: 0.15em; }
.radar-caption .count { font-family: "Gaydar Display", sans-serif; font-weight: 600; font-size: 15px; color: var(--pink); }

.spacer { flex: 1; }
.guest { position: relative; background: linear-gradient(135deg, #f6e6d8 0%, #ffe9dd 48%, #f6cfdb 100%); color: #170a14;
  padding: 13px 12px 11px 13px; transform: rotate(-1.4deg); border-radius: 2px;
  box-shadow: 0 14px 34px rgba(0,0,0,0.42); }
.guest:before, .guest:after { content: ""; position: absolute; top: 73px; width: 12px; height: 12px; border-radius: 6px; background: var(--bg); }
.guest:before { left: -7px; } .guest:after { right: -7px; }
.guest-top { display: flex; justify-content: space-between; align-items: flex-start; }
.guest-title { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 25px; line-height: 0.92; letter-spacing: -0.01em; }
.guest-title .g2 { color: var(--deep); }
.guest-heart { width: 13px; height: 13px; background: var(--deep); margin-top: 2px; }
.guest-rule { margin-top: 8px; border-top: 1px dashed rgba(23,10,20,0.3); }
.guest-sub { margin-top: 7px; font-family: "IBM Plex Mono", monospace; font-size: 6.6px; letter-spacing: 0.13em; color: rgba(23,10,20,0.62); }
.guest-no { margin-top: 4px; font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 13px; letter-spacing: 0.06em; }
.guest-no .hl { background: var(--pink); color: #1a0814; padding: 1px 3px; }
.guest-foot { display: flex; align-items: flex-end; justify-content: space-between; margin-top: 9px; }
.barcode { display: flex; gap: 2px; align-items: stretch; height: 20px; }
.barcode div { background: #170a14; }
.guest-code { font-family: "IBM Plex Mono", monospace; font-size: 5.8px; letter-spacing: 0.08em; color: rgba(23,10,20,0.55); }
.guest-holo { position: absolute; right: -7px; bottom: 34px; width: 28px; height: 17px; border-radius: 9px;
  background: linear-gradient(120deg, rgba(255,79,163,0.75), rgba(89,230,255,0.65), rgba(201,255,92,0.7)); opacity: 0.5; }

.alsa { margin-top: 14px; height: 46px; border: 1px solid var(--line); border-radius: 3px; padding: 0 10px;
  display: flex; align-items: center; gap: 8px; background: rgba(255,243,233,0.025); }
.alsa-dot { width: 5px; height: 5px; border-radius: 3px; background: var(--lime); box-shadow: 0 0 8px rgba(201,255,92,0.8); animation: pulse 2.4s ease-in-out infinite; }
.alsa-main { display: flex; flex-direction: column; gap: 2px; flex: 1; }
.alsa-title { font-family: "IBM Plex Mono", monospace; font-size: 7.5px; letter-spacing: 0.15em; color: var(--cream); }
.alsa-sub { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.09em; color: var(--dim); }
.alsa-bars { display: flex; align-items: flex-end; gap: 2px; height: 13px; }
.alsa-bars div { width: 2px; background: var(--lime); animation: eq1 1.2s ease-in-out infinite alternate; }
.rail-foot { margin-top: 14px; display: flex; align-items: center; justify-content: space-between; }
.flag { display: flex; flex-direction: column; width: 26px; height: 16px; overflow: hidden; border: 1px solid rgba(0,0,0,0.25); }
.flag div { flex: 1; }
.flag .f1 { background: #ff4fa3; } .flag .f2 { background: #ff8a5c; } .flag .f3 { background: #ffe066; }
.flag .f4 { background: #b6ff5c; } .flag .f5 { background: #59e6ff; } .flag .f6 { background: #c7a6ff; }
.foot-note { font-family: "IBM Plex Mono", monospace; font-size: 6.2px; letter-spacing: 0.10em; color: var(--dim); line-height: 1.55; text-align: right; }

.center { width: 916px; height: 900px; flex-shrink: 0; position: relative; display: flex; flex-direction: column; }
.topbar { height: 62px; flex-shrink: 0; border-bottom: 1px solid var(--line); display: flex; align-items: center;
  padding: 0 34px; justify-content: space-between; background: rgba(9,5,13,0.22); }
.crumb { display: flex; align-items: center; gap: 9px; }
.crumb .bar { width: 18px; height: 2px; background: var(--pink); }
.crumb .place { font-family: "IBM Plex Mono", monospace; font-size: 7.5px; letter-spacing: 0.17em; color: var(--cream); }
.live-pill { display: flex; align-items: center; gap: 7px; height: 26px; padding: 0 12px; border: 1px solid rgba(255,79,163,0.25);
  border-radius: 14px; background: rgba(255,79,163,0.06); }
.live-pill .dot { width: 5px; height: 5px; border-radius: 3px; margin: 0; background: var(--pink); box-shadow: 0 0 8px rgba(255,79,163,0.9); animation: pulse 1.8s ease-in-out infinite; }
.live-pill .txt { font-family: "IBM Plex Mono", monospace; font-size: 7px; letter-spacing: 0.18em; color: var(--cream); }
.live-pill .time { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.06em; color: var(--muted); }
.top-right { display: flex; align-items: center; gap: 13px; }
.serving-pill { display: flex; align-items: center; gap: 6px; height: 22px; padding: 0 8px; border: 1px solid rgba(201,255,92,0.28);
  border-radius: 2px; }
.serving-pill .txt { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.16em; color: var(--lime); }
.serving-pill .mini-bars { display: flex; align-items: flex-end; gap: 1.5px; height: 8px; }
.serving-pill .mini-bars div { width: 1.5px; background: var(--lime); animation: eq1 1s ease-in-out infinite alternate; }
.top-time { font-family: "IBM Plex Mono", monospace; font-size: 8px; letter-spacing: 0.12em; color: var(--cream); }
.top-time .tz { color: var(--dim); font-size: 6px; }
.avatar { position: relative; width: 27px; height: 27px; border-radius: 14px;
  background: linear-gradient(140deg, #ff8ac1, #ff4fa3 45%, #7a2a63); display: flex; align-items: center; justify-content: center; }
.avatar .letter { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 12px; color: #200a16; }
.avatar .status { position: absolute; right: -1px; bottom: -1px; width: 7px; height: 7px; border-radius: 4px; background: var(--lime); border: 1.5px solid #0b0611; }

.hero { position: relative; height: 424px; flex-shrink: 0; overflow: hidden; }
.art { position: absolute; left: 36px; top: 26px; width: 372px; height: 372px; border-radius: 4px; overflow: hidden;
  transform: rotate(-1.3deg); z-index: 4; border: 1px solid rgba(255,243,233,0.38); border-bottom: 0;
  box-shadow: 0 34px 70px rgba(0,0,0,0.62), 0 0 0 1px rgba(0,0,0,0.5); background: #ff4fa3; }
.art-base { position: absolute; left: 0; top: 0; width: 372px; height: 372px;
  background: linear-gradient(152deg, #ffc0da 0%, #ff79b3 18%, #f24d8f 40%, #dd4268 56%, #7f276b 76%, #260a2c 94%, #17051d 100%); }
.art-glow { position: absolute; left: -90px; top: -110px; width: 400px; height: 380px;
  background: radial-gradient(circle at 44% 40%, rgba(255,243,233,0.34) 0%, rgba(255,243,233,0.07) 30%, rgba(255,243,233,0) 60%); }
.art-glow2 { position: absolute; left: 110px; top: 205px; width: 350px; height: 280px;
  background: radial-gradient(circle, rgba(255,79,163,0.26) 0%, rgba(255,79,163,0) 68%); }
.art-rays { position: absolute; left: 186px; top: 178px; width: 0; height: 0; opacity: 0.8; }
.art-rays .ray { position: absolute; left: -1px; bottom: 0; width: 2px; height: 260px; transform-origin: 50% 100%;
  background: linear-gradient(to top, rgba(255,243,233,0.24), rgba(255,243,233,0.03) 62%, rgba(255,243,233,0) 100%); }
.art-orbit { position: absolute; left: 58px; top: 50px; width: 256px; height: 256px; border-radius: 128px;
  border: 1px solid rgba(255,243,233,0.26); }
.art-orbit2 { position: absolute; left: 79px; top: 71px; width: 214px; height: 214px; border-radius: 107px;
  border: 1px dashed rgba(255,243,233,0.18); }
.art-orbit-dot { position: absolute; left: 306px; top: 116px; width: 5px; height: 5px; border-radius: 3px; background: var(--cream); box-shadow: 0 0 8px rgba(255,243,233,0.9); }
.heart-rim { position: absolute; left: 88px; top: 84px; width: 196px; height: 184px;
  background: linear-gradient(160deg, rgba(255,243,233,0.92), rgba(255,185,218,0.65) 60%, rgba(255,79,163,0.55)); }
.heart-wrap { position: absolute; left: 94px; top: 90px; width: 184px; height: 172px;
  filter: drop-shadow(0 20px 26px rgba(45,4,25,0.55)); }
.chrome-heart-shadow { position: absolute; left: 102px; top: 98px; width: 184px; height: 172px;
  background: rgba(89,230,255,0.30); filter: blur(5px); }
.heart-sculpt { position: absolute; left: 0; top: 0; width: 184px; height: 172px; }
.chrome-heart { position: absolute; left: 0; top: 0; width: 184px; height: 172px;
  background: linear-gradient(150deg, #fffdf8 0%, #ffe9f5 7%, #ffb3d4 18%, #ff5cae 34%, #c21662 52%, #4a0a2c 66%, #a3185c 79%, #ff8ac1 90%, #ffe2ef 100%); }
.chrome-heart-gloss { position: absolute; left: -14px; top: 38px; width: 212px; height: 30px;
  background: linear-gradient(90deg, rgba(255,255,255,0), rgba(255,255,255,0.72), rgba(255,255,255,0.05));
  transform: rotate(-28deg); filter: blur(0.5px); }
.chrome-heart-spec { position: absolute; left: 32px; top: 12px; width: 62px; height: 48px; border-radius: 50%;
  background: radial-gradient(ellipse at 42% 36%, rgba(255,255,255,0.96) 0%, rgba(255,255,255,0.20) 54%, rgba(255,255,255,0) 76%); }
.art-spark { position: absolute; left: 262px; top: 92px; width: 13px; height: 13px; background: var(--cream);
  clip-path: polygon(50% 0%, 60% 40%, 100% 50%, 60% 60%, 50% 100%, 40% 60%, 0% 50%, 40% 40%);
  opacity: 0.8; animation: spin 16s linear infinite; }
.art-shade { position: absolute; left: 0; top: 0; width: 372px; height: 372px;
  background: linear-gradient(to top, rgba(13,4,17,0.92) 0%, rgba(13,4,17,0.55) 15%, rgba(13,4,17,0.16) 38%, rgba(13,4,17,0) 58%); }
.art-label { position: absolute; font-family: "IBM Plex Mono", monospace; font-size: 6.6px; letter-spacing: 0.16em;
  color: rgba(255,243,233,0.88); }
.art-label.tl { left: 14px; top: 12px; color: rgba(23,10,20,0.62); }
.art-label.tr { right: 14px; top: 12px; text-align: right; }
.art-rule { position: absolute; left: 14px; top: 24px; width: 40px; height: 1px; background: rgba(23,10,20,0.55); }
.art-shine { position: absolute; left: -120px; top: 250px; width: 560px; height: 50px; transform: rotate(-28deg);
  background: linear-gradient(90deg, rgba(255,255,255,0), rgba(255,255,255,0.12), rgba(255,255,255,0)); }
.art-scan { position: absolute; left: 0; top: 0; width: 372px; height: 372px;
  background: repeating-linear-gradient(180deg, rgba(255,255,255,0.03) 0px, rgba(255,255,255,0.03) 1px, rgba(0,0,0,0) 1px, rgba(0,0,0,0) 4px); }

.vinyl { position: absolute; left: 300px; top: 120px; width: 350px; height: 350px; border-radius: 175px; z-index: 2;
  overflow: hidden; border: 2px solid rgba(255,243,233,0.56);
  background: radial-gradient(circle at 35% 26%, #3c2745 0%, #211428 38%, #0c0712 72%, #040207 100%);
  box-shadow: 0 0 0 1px rgba(0,0,0,0.60), 0 32px 68px rgba(0,0,0,0.60), 0 0 30px rgba(255,243,233,0.045); }
.vinyl-spin { position: absolute; left: 0; top: 0; width: 350px; height: 350px; border-radius: 175px;
  transform-origin: 50% 50%; animation: spin 32s linear infinite; opacity: 0.85;
  background: repeating-radial-gradient(circle, #412a49 0px, #412a49 1.5px, #0a0610 2px, #0a0610 5px); }
.vinyl-cue { position: absolute; left: 171px; top: 16px; width: 5px; height: 5px; border-radius: 3px;
  background: rgba(255,243,233,0.55); box-shadow: 0 0 6px rgba(255,243,233,0.5); }
.groove { position: absolute; border-radius: 50%; border: 1px solid rgba(255,243,233,0.075); }
.vinyl-arc { position: absolute; left: 26px; top: 26px; width: 298px; height: 298px; border-radius: 149px;
  border: 1px solid transparent; border-top-color: rgba(255,243,233,0.22); transform: rotate(-36deg); z-index: 3; }
.vinyl-arc2 { position: absolute; left: 40px; top: 40px; width: 270px; height: 270px; border-radius: 135px;
  border: 1px solid transparent; border-top-color: rgba(89,230,255,0.16); transform: rotate(146deg); z-index: 3; }
.vinyl-rim { position: absolute; left: 5px; top: 5px; width: 340px; height: 340px; border-radius: 170px;
  border: 1px solid rgba(255,243,233,0.16); z-index: 4; }
.vinyl-ring { position: absolute; left: 0; top: 0; width: 100%; height: 100%; border-radius: 50%;
  border: 2px solid rgba(255,243,233,0.52); box-shadow: inset 0 0 0 1px rgba(0,0,0,0.55); }
.vinyl-tint { position: absolute; left: 0; top: 0; width: 350px; height: 350px; border-radius: 175px;
  background: linear-gradient(118deg, rgba(255,255,255,0.075) 0%, rgba(255,255,255,0.010) 28%, rgba(0,0,0,0) 50%),
              linear-gradient(300deg, rgba(89,230,255,0.030) 0%, rgba(0,0,0,0) 42%),
              radial-gradient(circle at 34% 30%, rgba(255,138,193,0.040) 0%, rgba(255,138,193,0) 55%); }
.vinyl-label { position: absolute; left: 122px; top: 122px; width: 106px; height: 106px; border-radius: 53px;
  background: radial-gradient(circle at 36% 28%, #ff9ccb 0%, #ff4fa3 46%, #d62b7a 100%);
  display: flex; flex-direction: column; z-index: 5;
  align-items: center; justify-content: center; gap: 3px; border: 1px solid rgba(255,243,233,0.55);
  box-shadow: inset 0 0 0 3px rgba(255,243,233,0.16), 0 4px 12px rgba(0,0,0,0.40); }
.label-gloss { position: absolute; left: 9px; top: 7px; width: 88px; height: 45px; border-radius: 50%;
  background: linear-gradient(135deg, rgba(255,255,255,0.35), rgba(255,255,255,0) 72%); }
.vinyl-label .l1 { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 11px; letter-spacing: 0.14em; color: #200a16; }
.vinyl-label .l2 { font-family: "IBM Plex Mono", monospace; font-size: 4.3px; letter-spacing: 0.15em; color: rgba(32,10,22,0.65); }
.vinyl-glint { position: absolute; left: -90px; top: 40px; width: 420px; height: 70px; transform: rotate(32deg); z-index: 7;
  background: linear-gradient(90deg, rgba(255,255,255,0), rgba(255,255,255,0.14), rgba(255,255,255,0)); }

.cert { position: absolute; left: 340px; top: 6px; width: 92px; height: 92px; border-radius: 46px; z-index: 8;
  transform: rotate(7deg); background: var(--cream); color: #170a14; display: flex; flex-direction: column;
  align-items: center; justify-content: center; gap: 2px; box-shadow: 0 10px 24px rgba(0,0,0,0.4);
  border: 1px solid rgba(23,10,20,0.35); }
.cert-inner { position: absolute; left: 4px; top: 4px; width: 84px; height: 84px; border-radius: 42px;
  border: 1px dashed rgba(23,10,20,0.45); }
.cert .c1 { font-family: "IBM Plex Mono", monospace; font-size: 5.2px; letter-spacing: 0.19em; color: rgba(23,10,20,0.62); }
.cert .c2 { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 12px; letter-spacing: 0.08em; }
.cert .c3 { display: flex; align-items: center; gap: 3px; font-family: "IBM Plex Mono", monospace; font-size: 6px; letter-spacing: 0.06em; }
.cert .mini-heart { width: 6px; height: 6px; background: var(--pink); }

.title-block { position: absolute; left: 560px; top: 24px; width: 340px; z-index: 6; }
.overline { display: flex; align-items: center; gap: 8px; }
.overline .dash { width: 26px; height: 1px; background: var(--pink); }
.overline .mono { font-size: 7px; letter-spacing: 0.22em; color: var(--pink); }
.overline .right { font-size: 6.6px; letter-spacing: 0.16em; color: var(--muted); }
.title { margin-top: 14px; font-family: "Gaydar Display", "IBM Plex Sans", sans-serif; font-weight: 700;
  font-size: 68px; line-height: 0.84; letter-spacing: -0.012em; color: var(--cream); white-space: nowrap;
  text-shadow: 3px 3px 0 rgba(214,43,122,0.22); }
.title-line { display: block; }
.title .pink { color: var(--pink); }
.artist-row { margin-top: 17px; display: flex; align-items: center; gap: 9px; }
.artist { font-family: "Gaydar Serif", "IBM Plex Serif", serif; font-style: italic; font-weight: 300; font-size: 29px;
  line-height: 1; color: var(--cream); }
.artist-seal { width: 13px; height: 13px; border-radius: 7px; background: var(--pink); display: flex; align-items: center; justify-content: center; }
.artist-seal .check { width: 5px; height: 3px; border-left: 1px solid #1a0814; border-bottom: 1px solid #1a0814; transform: rotate(-45deg); margin-top: -1px; }
.release { margin-top: 8px; font-family: "IBM Plex Mono", monospace; font-size: 7px; letter-spacing: 0.14em; color: var(--muted); }
.stats { display: flex; gap: 0; margin-top: 20px; border-top: 1px solid var(--line); padding-top: 11px; width: 340px; }
.stat { width: 85px; display: flex; flex-direction: column; gap: 4px; }
.stat .k { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.15em; color: var(--dim); }
.stat .v { font-family: "IBM Plex Mono", monospace; font-size: 9.5px; letter-spacing: 0.06em; color: var(--cream); }
.stat .v.pink { color: var(--pink); }
.tags { display: flex; gap: 5px; margin-top: 14px; }
.tag { height: 18px; padding: 0 7px; display: flex; align-items: center; border: 1px solid var(--line);
  border-radius: 2px; font-family: "IBM Plex Mono", monospace; font-size: 5.6px; letter-spacing: 0.12em; color: var(--muted); }
.tag.hot { border-color: rgba(255,79,163,0.35); color: var(--pink); }

.phrase-wrap { position: absolute; left: 100px; top: 322px; width: 760px; z-index: 9; transform: rotate(-2.1deg); }
.phrase-label { display: flex; align-items: center; gap: 7px; margin-bottom: 3px; margin-left: 4px; }
.phrase-label .mono { font-size: 6.8px; letter-spacing: 0.2em; color: rgba(255,243,233,0.86); }
.phrase-label .l { width: 16px; height: 1px; background: rgba(255,243,233,0.45); }
.phrase { font-family: "Gaydar Serif", "IBM Plex Serif", serif; font-style: italic; font-weight: 300; font-size: 50px;
  line-height: 1.04; color: var(--cream); white-space: nowrap; text-shadow: 0 3px 24px rgba(37,3,22,0.55); }
.phrase-w { display: inline-block; margin-right: 0.30em; }
.phrase-w.end { margin-right: 0; }
.phrase .pink { color: #ff9ccb; border-bottom: 3px solid var(--pink); padding-bottom: 2px; }

.deck { height: 300px; flex-shrink: 0; border-top: 1px solid var(--line); position: relative;
  padding: 16px 34px 0 34px;
  background: linear-gradient(180deg, rgba(9,5,13,0.55) 0%, rgba(9,5,13,0.34) 34%, rgba(9,5,13,0.68) 100%); }
.deck-head { display: flex; justify-content: space-between; align-items: center; height: 14px; }
.deck-head .mono { font-size: 6.8px; letter-spacing: 0.18em; }
.wave-row { margin-top: 8px; display: flex; align-items: center; gap: 12px; height: 58px; }
.time { font-family: "IBM Plex Mono", monospace; font-size: 9px; letter-spacing: 0.06em; color: var(--muted); width: 30px; }
.time.now { color: var(--cream); text-align: right; }
.wave { flex: 1; height: 56px; display: flex; align-items: center; gap: 3px; position: relative; }
.wbar { width: 5px; border-radius: 2px; flex-shrink: 0; }
.playhead { position: absolute; left: 474px; top: -2px; width: 2px; height: 60px; background: var(--cream);
  box-shadow: 0 0 12px rgba(255,243,233,0.75); }
.playhead .knob { position: absolute; left: -3px; top: -4px; width: 8px; height: 8px; border-radius: 4px; background: var(--cream); }

.controls { margin-top: 24px; display: flex; align-items: center; }
.ctrl-side { width: 226px; display: flex; align-items: center; gap: 10px; }
.ctrl-side.right { justify-content: flex-end; }
.ctrl-center { flex: 1; display: flex; align-items: center; justify-content: center; gap: 22px; }
.mini-btn { display: flex; align-items: center; gap: 6px; padding: 6px 7px; border-radius: 3px; cursor: pointer;
  color: var(--cream); transition: background 0.2s ease; }
.mini-btn.on { color: var(--pink); }
.mini-btn:hover { background: rgba(255,243,233,0.05); }
.mini-btn .lbl { font-family: "IBM Plex Mono", monospace; font-size: 6.3px; letter-spacing: 0.14em; color: var(--dim); }
.mini-btn.on .lbl { color: var(--pink); }
.ic-shuffle { position: relative; width: 16px; height: 14px; }
.ic-shuffle .l1 { position: absolute; left: 1px; top: 3px; width: 14px; height: 1.2px; background: currentColor; transform: rotate(20deg); }
.ic-shuffle .l2 { position: absolute; left: 1px; top: 10px; width: 14px; height: 1.2px; background: currentColor; transform: rotate(-20deg); }
.ic-shuffle .a1 { position: absolute; right: 0; top: 1px; width: 3px; height: 3px; border-top: 1px solid currentColor; border-right: 1px solid currentColor; }
.ic-shuffle .a2 { position: absolute; right: 0; bottom: 1px; width: 3px; height: 3px; border-bottom: 1px solid currentColor; border-right: 1px solid currentColor; }
.ic-repeat { position: relative; width: 16px; height: 14px; }
.ic-repeat .loop { position: absolute; left: 1px; top: 3px; width: 13px; height: 8px; border: 1.2px solid currentColor; border-right-color: transparent; border-radius: 3px; }
.ic-repeat .a { position: absolute; right: 0; top: 1px; width: 4px; height: 4px; border-top: 1.2px solid currentColor; border-right: 1.2px solid currentColor; }
.ic-prev, .ic-next { position: relative; width: 26px; height: 22px; cursor: pointer; opacity: 0.85; transition: opacity 0.2s ease; }
.ic-prev:hover, .ic-next:hover { opacity: 1; }
.ic-prev .tri { position: absolute; left: 7px; top: 4px; width: 9px; height: 14px; background: var(--cream); clip-path: polygon(100% 0%, 100% 100%, 0% 50%); }
.ic-prev .bar { position: absolute; left: 2px; top: 4px; width: 2px; height: 14px; background: var(--cream); }
.ic-next .tri { position: absolute; left: 10px; top: 4px; width: 9px; height: 14px; background: var(--cream); clip-path: polygon(0% 0%, 0% 100%, 100% 50%); }
.ic-next .bar { position: absolute; right: 2px; top: 4px; width: 2px; height: 14px; background: var(--cream); }
.play-wrap { position: relative; width: 80px; height: 80px; display: flex; align-items: center; justify-content: center; }
.play-ring { position: absolute; left: 3px; top: 3px; width: 74px; height: 74px; border-radius: 37px;
  border: 1px solid rgba(255,79,163,0.45); animation: breathe 3.2s ease-in-out infinite; }
.play { width: 64px; height: 64px; border-radius: 32px; cursor: pointer;
  background: linear-gradient(145deg, #ff9ccb 0%, #ff4fa3 42%, #d62b7a 100%);
  display: flex; align-items: center; justify-content: center;
  box-shadow: 0 12px 34px rgba(255,79,163,0.42), inset 0 1px 0 rgba(255,255,255,0.55);
  transition: transform 0.2s ease; }
.play:hover { transform: scale(1.04); }
.play .tri { width: 17px; height: 21px; background: #1a0814; clip-path: polygon(0% 0%, 0% 100%, 100% 50%); margin-left: 4px; }
.play .pause { display: flex; gap: 5px; }
.play .pause div { width: 5px; height: 19px; border-radius: 1px; background: #1a0814; }
.like { display: flex; align-items: center; gap: 7px; cursor: pointer; }
.like-btn { width: 34px; height: 34px; border-radius: 17px; border: 1px solid var(--line); display: flex; align-items: center; justify-content: center; }
.like-btn .mini-heart { width: 12px; height: 11px; background: rgba(255,243,233,0.18); }
.like.on .like-btn { border-color: rgba(255,79,163,0.55); background: rgba(255,79,163,0.10); }
.like.on .mini-heart { background: var(--pink); box-shadow: 0 0 8px rgba(255,79,163,0.6); }
.like-meta { display: flex; flex-direction: column; gap: 2px; }
.like-meta .n { font-family: "IBM Plex Mono", monospace; font-size: 8px; letter-spacing: 0.04em; color: var(--cream); }
.like-meta .k { font-family: "IBM Plex Mono", monospace; font-size: 5.2px; letter-spacing: 0.14em; color: var(--dim); }
.vol { display: flex; align-items: center; gap: 8px; margin-left: 8px; }
.vol .k { font-family: "IBM Plex Mono", monospace; font-size: 6.2px; letter-spacing: 0.14em; color: var(--dim); }
.vol-track { position: relative; width: 74px; height: 3px; border-radius: 2px; background: rgba(255,243,233,0.13); }
.vol-fill { position: absolute; left: 0; top: 0; height: 3px; border-radius: 2px; background: linear-gradient(90deg, var(--lilac), var(--pink)); }
.vol-knob { position: absolute; top: -3px; width: 9px; height: 9px; border-radius: 5px; background: var(--cream); box-shadow: 0 0 6px rgba(255,243,233,0.4); }
.vol .v { font-family: "IBM Plex Mono", monospace; font-size: 7.5px; color: var(--cream); }
.deck-foot { margin-top: 18px; display: flex; align-items: center; justify-content: space-between; }
.deck-tags { display: flex; gap: 6px; }
.deck-tags .tag { height: 17px; font-size: 6px; }
.deck-quality { display: flex; align-items: center; gap: 8px; }
.deck-quality .mono { font-size: 6.2px; letter-spacing: 0.12em; }
.hd-badge { width: 17px; height: 12px; border: 1px solid var(--lime); display: flex; align-items: center; justify-content: center;
  font-family: "IBM Plex Mono", monospace; font-size: 5.4px; letter-spacing: 0.04em; color: var(--lime); }

.notes { height: 70px; flex-shrink: 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line);
  display: flex; align-items: center; padding: 0 34px; gap: 22px; background: rgba(255,243,233,0.014); }
.notes .quote-mark { font-family: "Gaydar Serif", serif; font-style: italic; font-size: 40px; line-height: 0.7; color: var(--pink); opacity: 0.9; }
.notes-copy { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.notes-copy .k { font-family: "IBM Plex Mono", monospace; font-size: 5.7px; letter-spacing: 0.18em; color: var(--pink); }
.notes-copy .q { font-family: "Gaydar Serif", "IBM Plex Serif", serif; font-style: italic; font-weight: 300; font-size: 16.5px; color: #e9d9e2; }
.notes-copy .q .pink { color: var(--pink2); }
.eq-block { display: flex; align-items: center; gap: 10px; padding-left: 20px; border-left: 1px solid var(--line2); }
.eq { display: flex; align-items: flex-end; gap: 4px; height: 30px; }
.eq div { width: 4px; border-radius: 1px; transform-origin: bottom; }
.eq .e1 { height: 30px; background: var(--pink); animation: eq1 1.3s ease-in-out infinite alternate; }
.eq .e2 { height: 30px; background: var(--lilac); animation: eq2 0.9s ease-in-out infinite alternate; }
.eq .e3 { height: 30px; background: var(--cyan); animation: eq3 1.1s ease-in-out infinite alternate; }
.eq .e4 { height: 30px; background: var(--lime); animation: eq4 0.8s ease-in-out infinite alternate; }
.eq-label { display: flex; flex-direction: column; gap: 3px; }
.eq-label .a { font-family: "IBM Plex Mono", monospace; font-size: 6.8px; letter-spacing: 0.15em; color: var(--cream); }
.eq-label .b { font-family: "IBM Plex Mono", monospace; font-size: 5.8px; letter-spacing: 0.12em; color: var(--dim); }
.serve-stamp { display: flex; flex-direction: column; align-items: flex-end; gap: 3px; padding-left: 20px; border-left: 1px solid var(--line2); }
.serve-stamp .a { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 15px; letter-spacing: 0.10em; color: var(--lime); line-height: 1; }
.serve-stamp .b { font-family: "IBM Plex Mono", monospace; font-size: 5.4px; letter-spacing: 0.14em; color: var(--dim); }

.ticker { height: 44px; flex-shrink: 0; display: flex; align-items: center; padding: 0 20px; gap: 12px;
  background: rgba(0,0,0,0.18); }
.ticker .tagline { font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.16em; color: var(--pink); white-space: nowrap; }
.marquee { flex: 1; overflow: hidden; height: 12px; position: relative; }
.marquee-track { display: flex; white-space: nowrap; animation: marquee 34s linear infinite; }
.marquee-track .chunk { display: flex; align-items: center; font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.15em; color: var(--muted); }
.marquee-track .titem { display: inline-block; margin-right: 14px; }
.marquee-track .sep { width: 3px; height: 3px; border-radius: 2px; background: var(--pink); margin: 0; flex-shrink: 0; }
.ticker-right { display: flex; align-items: center; gap: 9px; }
.ticker-right .dot { width: 4px; height: 4px; border-radius: 2px; margin: 0; background: var(--lime); }
.ticker-right .t { font-family: "IBM Plex Mono", monospace; font-size: 6.4px; letter-spacing: 0.14em; color: var(--muted); white-space: nowrap; }

.rail-right { width: 296px; height: 900px; flex-shrink: 0; border-left: 1px solid var(--line);
  background: linear-gradient(270deg, rgba(9,5,13,0.965), rgba(9,5,13,0.90) 55%, rgba(9,5,13,0.82)); padding: 24px 22px 18px 22px;
  display: flex; flex-direction: column; }
.rail-head { display: flex; align-items: baseline; justify-content: space-between; }
.rail-head .title { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 14px; letter-spacing: 0.14em; color: var(--cream); }
.rail-head .count { font-family: "IBM Plex Mono", monospace; font-size: 6px; letter-spacing: 0.14em; color: var(--pink); }
.rail-head2 { margin-top: 4px; display: flex; justify-content: space-between; }
.rail-head2 .mono { font-size: 5.6px; letter-spacing: 0.14em; }
.queue { margin-top: 12px; }
.qrow { position: relative; height: 70px; display: flex; align-items: center; gap: 9px; border-top: 1px solid var(--line2);
  cursor: pointer; transition: background 0.2s ease; }
.qrow:hover { background: rgba(255,243,233,0.03); }
.qrow.first { border-top-color: var(--line); background: linear-gradient(90deg, rgba(255,79,163,0.09), rgba(255,79,163,0) 85%); }
.qrow.first:before { content: ""; position: absolute; left: -22px; top: 14px; width: 2px; height: 42px; background: var(--pink); }
.qnum { width: 14px; font-family: "IBM Plex Mono", monospace; font-size: 6.5px; letter-spacing: 0.03em; color: var(--dim); }
.qrow.first .qnum { color: var(--pink); }
.qart { position: relative; width: 42px; height: 42px; border-radius: 3px; overflow: hidden; flex-shrink: 0; border: 1px solid rgba(255,243,233,0.16); }
.qinfo { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
.qtitle { font-size: 12px; font-weight: 600; letter-spacing: 0.01em; color: var(--cream); white-space: nowrap; overflow: hidden; }
.qartist { font-family: "IBM Plex Mono", monospace; font-size: 7.2px; letter-spacing: 0.08em; color: var(--muted); white-space: nowrap; }
.qtime { font-family: "IBM Plex Mono", monospace; font-size: 7.2px; letter-spacing: 0.03em; color: var(--dim); }
.qa { position: absolute; }
.qa-circle { left: 9px; top: 9px; width: 24px; height: 24px; border: 2px solid rgba(255,243,233,0.85); border-radius: 12px; }
.qa-bar { left: 0; top: 19px; width: 42px; height: 6px; background: #0b060f; opacity: 0.75; }
.qa-sq { left: 10px; top: 10px; width: 22px; height: 22px; background: #0b060f; transform: rotate(18deg); }
.qa-heart { left: 12px; top: 12px; width: 19px; height: 18px; background: #1a0814; }
.qa-star { left: 12px; top: 12px; width: 18px; height: 18px; background: var(--cream);
  clip-path: polygon(50% 0%, 60% 40%, 100% 50%, 60% 60%, 50% 100%, 40% 60%, 0% 50%, 40% 40%); }
.qart-1 { background: linear-gradient(150deg, #59e6ff 0%, #2d6fbf 48%, #1d0f4a 100%); }
.qart-2 { background: linear-gradient(150deg, #c9ff5c 0%, #3d7a4f 55%, #0a241b 100%); }
.qart-3 { background: linear-gradient(150deg, #c7a6ff 0%, #ff4fa3 62%, #4a0a2d 100%); }
.qart-4 { background: linear-gradient(150deg, #ff8a5c 0%, #c62b6c 55%, #2a0c2e 100%); }

.conf { margin-top: 24px; padding-top: 13px; border-top: 1px solid var(--line); }
.conf-head { display: flex; justify-content: space-between; }
.conf-head .lbl { font-family: "IBM Plex Mono", monospace; font-size: 6.2px; letter-spacing: 0.18em; color: var(--cream); }
.conf-head .live { font-family: "IBM Plex Mono", monospace; font-size: 5.4px; letter-spacing: 0.13em; color: var(--lime); }
.conf-main { display: flex; align-items: flex-end; gap: 3px; margin-top: 4px; }
.conf-num { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 56px; line-height: 0.8; color: var(--cream); }
.conf-pct { font-family: "Gaydar Display", sans-serif; font-weight: 700; font-size: 17px; color: var(--pink); margin-bottom: 5px; }
.conf-match { font-family: "IBM Plex Mono", monospace; font-size: 5.8px; letter-spacing: 0.16em; color: var(--dim); margin-bottom: 8px; margin-left: 3px; }
.conf-meter { display: flex; gap: 2px; margin-top: 9px; height: 12px; align-items: stretch; }
.conf-seg { width: 7px; border-radius: 1px; background: rgba(255,243,233,0.08); }
.conf-seg.on { background: var(--pink); }
.conf-seg.on:nth-child(2n) { background: #ff74b8; }
.conf-seg.on:nth-child(3n) { background: var(--lilac); }
.conf-seg.on.glow { box-shadow: 0 0 8px rgba(255,79,163,0.75); }
.conf-note { margin-top: 9px; font-family: "Gaydar Serif", "IBM Plex Serif", serif; font-style: italic; font-size: 12.5px;
  line-height: 1.28; color: #c6b3c7; }
.conf-note .pink { color: var(--pink2); }

.mood { margin-top: 24px; padding-top: 13px; border-top: 1px solid var(--line); }
.mood-head { display: flex; justify-content: space-between; align-items: baseline; }
.mood-head .lbl { font-family: "IBM Plex Mono", monospace; font-size: 6.2px; letter-spacing: 0.18em; color: var(--cream); }
.mood-head .auto { font-family: "IBM Plex Mono", monospace; font-size: 5.2px; letter-spacing: 0.12em; color: var(--dim); }
.mood-rows { margin-top: 9px; display: flex; flex-direction: column; gap: 12px; }
.mood-row { display: flex; align-items: center; gap: 8px; }
.mood-name { width: 38px; font-family: "IBM Plex Mono", monospace; font-size: 7px; letter-spacing: 0.12em; color: var(--muted); }
.mood-track { position: relative; flex: 1; height: 7px; border-radius: 1px; background: rgba(255,243,233,0.07); overflow: visible; }
.mood-fill { position: absolute; left: 0; top: 0; height: 7px; border-radius: 1px; }
.mood-ticks { position: absolute; left: 0; top: 0; width: 100%; height: 7px;
  background: repeating-linear-gradient(90deg, transparent 0px, transparent 5px, rgba(9,5,13,0.65) 5px, rgba(9,5,13,0.65) 6px); }
.mood-knob { position: absolute; top: -2px; width: 4px; height: 11px; border-radius: 1px; background: var(--cream); box-shadow: 0 0 5px rgba(255,243,233,0.5); }
.mood-val { width: 18px; text-align: right; font-family: "IBM Plex Mono", monospace; font-size: 7px; color: var(--dim); }
.stream { margin-top: 22px; padding-top: 12px; border-top: 1px solid var(--line); }
.stream-head { display: flex; justify-content: space-between; align-items: baseline; }
.stream-head .lbl { font-family: "IBM Plex Mono", monospace; font-size: 6.6px; letter-spacing: 0.18em; color: var(--cream); }
.stream-head .val { font-family: "IBM Plex Mono", monospace; font-size: 5.4px; letter-spacing: 0.11em; color: var(--lime); }
.stream-wave { margin-top: 9px; display: flex; align-items: center; gap: 2px; height: 24px; }
.stream-wave div { width: 2px; border-radius: 1px; }
.stream-foot { margin-top: 7px; display: flex; justify-content: space-between; }
.stream-foot .mono { font-size: 5.8px; letter-spacing: 0.10em; }
.rail-space { flex: 1; }
.rail-foot { border-top: 1px solid var(--line); padding-top: 12px; display: flex; flex-direction: column; gap: 8px; }
.signal { display: flex; align-items: center; gap: 7px; }
.signal .lbl { font-family: "IBM Plex Mono", monospace; font-size: 5.4px; letter-spacing: 0.14em; color: var(--dim); }
.signal-bars { display: flex; align-items: flex-end; gap: 2px; height: 10px; }
.signal-bars div { width: 2px; background: var(--lime); }
.signal .val { font-family: "IBM Plex Mono", monospace; font-size: 5.4px; letter-spacing: 0.10em; color: var(--lime); }
.rail-foot-bottom { display: flex; align-items: center; justify-content: space-between; }
.rail-foot-bottom .fl { display: flex; flex-direction: column; width: 22px; height: 14px; overflow: hidden; }
.rail-foot-bottom .fl div { flex: 1; }
.rail-foot-bottom .note { font-family: "IBM Plex Mono", monospace; font-size: 5px; letter-spacing: 0.09em; color: var(--dim); line-height: 1.5; text-align: right; }
.rail-foot-bottom .ver { color: var(--pink); }

.particles { position: absolute; left: 0; top: 0; width: 1440px; height: 900px; z-index: 20; }
.p { position: absolute; animation: drift 9s ease-in-out infinite; }
.p-heart { background: var(--pink); clip-path: polygon(50% 20%, 58% 8%, 72% 2%, 86% 6%, 94% 20%, 95% 36%, 88% 50%, 50% 92%, 12% 50%, 5% 36%, 6% 20%, 14% 6%, 28% 2%, 42% 8%); }
.p-star { background: var(--lime); clip-path: polygon(50% 0%, 60% 40%, 100% 50%, 60% 60%, 50% 100%, 40% 60%, 0% 50%, 40% 40%); }
.p-cyan { background: var(--cyan); clip-path: polygon(50% 0%, 60% 40%, 100% 50%, 60% 60%, 50% 100%, 40% 60%, 0% 50%, 40% 40%); }
.p-lilac { background: var(--lilac); border-radius: 50%; }
.p-cream { background: var(--cream); clip-path: polygon(50% 0%, 60% 40%, 100% 50%, 60% 60%, 50% 100%, 40% 60%, 0% 50%, 40% 40%); }
.p-blur { filter: blur(1.5px); opacity: 0.5; }

@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
@keyframes sweep { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
@keyframes blip { 0%, 100% { opacity: 0.25; } 50% { opacity: 1; } }
@keyframes breathe { 0%, 100% { transform: scale(0.94); opacity: 0.35; } 50% { transform: scale(1.03); opacity: 0.8; } }
@keyframes eq1 { from { transform: scaleY(0.28); } to { transform: scaleY(1); } }
@keyframes eq2 { from { transform: scaleY(0.55); } to { transform: scaleY(0.22); } }
@keyframes eq3 { from { transform: scaleY(0.38); } to { transform: scaleY(0.92); } }
@keyframes eq4 { from { transform: scaleY(0.7); } to { transform: scaleY(0.3); } }
@keyframes drift { 0%, 100% { transform: translateY(0px) rotate(0deg); opacity: 0.25; }
  35% { transform: translateY(-13px) rotate(14deg); opacity: 0.9; }
  70% { transform: translateY(-4px) rotate(-8deg); opacity: 0.6; } }
@keyframes marquee { from { transform: translateX(0px); } to { transform: translateX(-50%); } }
"###;

const QUEUE: [(&str, &str, &str, &str); 4] = [
    ("SATURDAY CATHEDRAL", "VELVET STATIC", "3:12", "qart-1"),
    ("BOYFAILURE", "MIRRORBALL", "2:47", "qart-2"),
    ("HOLY WATER", "SERAPHIM CLUB", "3:26", "qart-3"),
    ("GIRLS LIKE US", "THE PINK ROOM", "2:58", "qart-4"),
];

const MOODS: [(&str, u32, &str); 4] = [
    ("CAMP", 86, "#ff4fa3"),
    ("CLUB", 64, "#59e6ff"),
    ("SOFT", 92, "#c7a6ff"),
    ("CHAOS", 71, "#c9ff5c"),
];

const PARTICLES: [(f32, f32, f32, &str, f32, f32); 21] = [
    (250.0, 812.0, 5.0, "p-star", -2.0, 8.0),
    (612.0, 94.0, 9.0, "p-star", -5.0, 11.0),
    (832.0, 132.0, 4.0, "p-lilac", -3.0, 9.0),
    (1092.0, 214.0, 7.0, "p-cyan", -7.0, 12.0),
    (1194.0, 512.0, 6.0, "p-heart", -1.0, 10.0),
    (1362.0, 702.0, 8.0, "p-cream", -4.0, 13.0),
    (704.0, 764.0, 6.0, "p-star", -8.0, 9.5),
    (520.0, 88.0, 5.0, "p-cyan", -6.0, 10.5),
    (986.0, 632.0, 5.0, "p-star", -2.5, 12.5),
    (286.0, 842.0, 5.0, "p-heart", -9.0, 11.5),
    (1352.0, 264.0, 6.0, "p-cyan", -3.5, 9.0),
    (888.0, 238.0, 4.0, "p-heart", -5.5, 12.0),
    (1076.0, 822.0, 5.0, "p-lilac", -7.5, 10.0),
    (466.0, 76.0, 5.0, "p-cream", -1.5, 13.5),
    (1240.0, 384.0, 4.0, "p-heart", -8.5, 9.0),
    (134.0, 250.0, 4.0, "p-star", -4.5, 11.0),
    (760.0, 58.0, 6.0, "p-lilac", -6.5, 8.5),
    (1140.0, 748.0, 5.0, "p-cyan", -0.5, 12.0),
    (708.0, 166.0, 5.0, "p-heart", -3.2, 10.5),
    (902.0, 452.0, 4.0, "p-star", -5.4, 9.0),
    (1112.0, 650.0, 4.0, "p-cyan", -2.2, 11.5),
];

struct NowPlaying {
    playing: bool,
    liked: bool,
    nav: usize,
}

fn rainbow(t: f32) -> (u32, u32, u32) {
    let stops: [(f32, f32, f32); 6] = [
        (255.0, 79.0, 163.0),
        (255.0, 138.0, 92.0),
        (255.0, 224.0, 102.0),
        (182.0, 255.0, 92.0),
        (89.0, 230.0, 255.0),
        (199.0, 166.0, 255.0),
    ];
    let t = t.clamp(0.0, 1.0);
    let pos = t * (stops.len() - 1) as f32;
    let i = (pos.floor() as usize).min(stops.len() - 2);
    let f = pos - i as f32;
    let a = stops[i];
    let b = stops[i + 1];
    (
        (a.0 + (b.0 - a.0) * f) as u32,
        (a.1 + (b.1 - a.1) * f) as u32,
        (a.2 + (b.2 - a.2) * f) as u32,
    )
}

fn rays(count: usize, class: &'static str, height: f32) -> Div {
    let mut wrap = div().class(class);
    for i in 0..count {
        let angle = 360.0 * i as f32 / count as f32;
        wrap = wrap.child(div().class("ray").style(format!(
            "transform: rotate({angle}deg); height: {height}px;"
        )));
    }
    wrap
}

fn nav_icon(i: usize, active: bool) -> Div {
    let tint = if active { "ic-pink" } else { "ic-muted" };
    match i {
        0 => div()
            .class(format!("icon {tint}"))
            .child(div().class("ic-heart heart-clip fill")),
        1 => div()
            .class(format!("icon {tint}"))
            .child(div().class("ic-radar"))
            .child(div().class("ic-radar2"))
            .child(div().class("ic-radar3")),
        2 => div()
            .class(format!("icon {tint}"))
            .child(div().class("ic-search"))
            .child(div().class("ic-search-h")),
        3 => div()
            .class(format!("icon {tint}"))
            .child(div().class("ic-heart heart-clip")),
        _ => div()
            .class(format!("icon {tint}"))
            .child(div().class("ic-clock"))
            .child(div().class("ic-clock-h"))
            .child(div().class("ic-clock-m")),
    }
}

fn queue_thumb(class: &str, deco: Div) -> Div {
    div().class(format!("qart {class}")).child(deco)
}

fn bar_color(i: usize, n: usize, played: bool) -> String {
    let t = i as f32 / (n - 1).max(1) as f32;
    let (r, g, b) = rainbow(t);
    if played {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else if i % 9 == 0 {
        "#5b4963".to_string()
    } else {
        "#493c51".to_string()
    }
}

fn wave_height(i: usize) -> f32 {
    let x = i as f32;
    let a = ((x * 0.71).sin() * 0.5 + 0.5) * ((x * 0.23 + 1.3).cos() * 0.5 + 0.5);
    let b = ((x * 0.17).sin() * 0.5 + 0.5).powf(1.6);
    let env = 0.62 + 0.38 * ((x / 94.0) * std::f32::consts::PI).sin();
    (7.0 + 33.0 * (a * 0.55 + b * 0.45)) * env
}

fn page_grain(b: Bounds<f32>, p: &mut Painter) {
    let mut seed: u64 = 0x243f_6a88_85a3_08d3;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f32) / (u32::MAX as f32)
    };
    for _ in 0..2600 {
        let x = b.origin.x + rnd() * 1440.0;
        let y = b.origin.y + rnd() * 900.0;
        let r = 0.35 + rnd() * 0.8;
        let a: u32 = 8 + (rnd() * 12.0) as u32;
        let color = if rnd() > 0.72 {
            rgba(0xff4fa300 | a)
        } else if rnd() > 0.82 {
            rgba(0x59e6ff00 | a)
        } else {
            rgba(0xfff3e900 | a)
        };
        p.oval(
            Bounds {
                origin: point(x, y),
                size: size(r * 2.0, r * 2.0),
            },
            color,
            rgba(0x00000000),
            0.0,
        );
    }
}

fn art_texture(b: Bounds<f32>, p: &mut Painter) {
    let (ox, oy) = (b.origin.x, b.origin.y);
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f32) / (u32::MAX as f32)
    };
    for _ in 0..520 {
        let x = ox + rnd() * 372.0;
        let y = oy + rnd() * 372.0;
        let r = 0.35 + rnd() * 0.9;
        p.oval(
            Bounds {
                origin: point(x, y),
                size: size(r * 2.0, r * 2.0),
            },
            rgba(0xfff3e91c),
            rgba(0x00000000),
            0.0,
        );
    }
    for gy in 0..13 {
        for gx in 0..23 {
            let x = ox + 18.0 + gx as f32 * 7.2;
            let y = oy + 268.0 + gy as f32 * 7.2;
            let r = 1.0 + ((gx * 7 + gy * 13) % 5) as f32 * 0.2;
            p.oval(
                Bounds {
                    origin: point(x, y),
                    size: size(r * 2.0, r * 2.0),
                },
                rgba(0xfff3e936),
                rgba(0x00000000),
                0.0,
            );
        }
    }
    for k in 0..7 {
        let x = ox + 310.0 - k as f32 * 9.0;
        p.line(
            &[(x, oy + 28.0), (x + 3.0, oy + 52.0)],
            rgba(0xfff3e938),
            1.0,
        );
    }
}

impl Render for NowPlaying {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nav_labels = ["FOR YOU", "GAYDAR", "SEARCH", "LIKED", "HISTORY"];
        let mut nav_rows = div().class("nav");
        for i in 0..5 {
            let active = self.nav == i;
            let extra = if active {
                div().class("live-dot")
            } else {
                div().style("width: 4px; height: 4px;")
            };
            nav_rows = nav_rows.child(
                div()
                    .class(if active {
                        "nav-item active"
                    } else {
                        "nav-item"
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.nav = i;
                        cx.notify();
                    }))
                    .child(div().class("n").child(format!("0{}", i + 1)))
                    .child(nav_icon(i, active))
                    .child(div().class("lbl").child(nav_labels[i]))
                    .child(extra),
            );
        }

        let mut wave = div().class("wave");
        let bars = 94usize;
        for i in 0..bars {
            let played = (i as f32 / bars as f32) < 0.62;
            wave = wave.child(div().class("wbar").style(format!(
                "height: {:.1}px; background: {};",
                wave_height(i),
                bar_color(i, bars, played)
            )));
        }
        wave = wave.child(div().class("playhead").child(div().class("knob")));

        let mut conf_segs = div().class("conf-meter");
        for i in 0..24 {
            let on = i < 23;
            let glow = i == 22;
            conf_segs = conf_segs.child(div().class(format!(
                "conf-seg{}{}",
                if on { " on" } else { "" },
                if glow { " glow" } else { "" }
            )));
        }

        let mut mood_rows = div().class("mood-rows");
        for (name, value, color) in MOODS {
            mood_rows = mood_rows.child(
                div()
                    .class("mood-row")
                    .child(div().class("mood-name").child(name))
                    .child(
                        div()
                            .class("mood-track")
                            .child(
                                div()
                                    .class("mood-fill")
                                    .style(format!("width: {value}%; background: {color};")),
                            )
                            .child(div().class("mood-ticks"))
                            .child(
                                div()
                                    .class("mood-knob")
                                    .style(format!("left: calc({value}% - 2px);")),
                            ),
                    )
                    .child(div().class("mood-val").child(format!("{value}"))),
            );
        }

        let mut queue = div().class("queue");
        for (i, (title, artist, time, cls)) in QUEUE.iter().enumerate() {
            let first = i == 0;
            let deco = match i {
                0 => div().class("qa qa-circle"),
                1 => div().class("qa qa-sq"),
                2 => div().class("qa qa-heart heart-clip"),
                _ => div().class("qa qa-star"),
            };
            queue = queue.child(
                div()
                    .class(if first { "qrow first" } else { "qrow" })
                    .child(div().class("qnum").child(format!("0{}", i + 1)))
                    .child(queue_thumb(cls, deco))
                    .child(
                        div()
                            .class("qinfo")
                            .child(div().class("qtitle").child(*title))
                            .child(div().class("qartist").child(*artist)),
                    )
                    .child(div().class("qtime").child(*time)),
            );
        }

        let mut barcode = div().class("barcode");
        for i in 0..18 {
            let w = match i % 5 {
                0 => 1,
                1 => 2,
                2 => 1,
                3 => 3,
                _ => 1,
            };
            barcode = barcode.child(div().style(format!("width: {w}px;")));
        }

        let mut particles = div().class("particles");
        for (x, y, s, cls, delay, dur) in PARTICLES {
            particles = particles.child(
                div()
                    .class(format!("p {cls}{}", if s > 7.0 { " p-blur" } else { "" }))
                    .style(format!(
                        "left: {x}px; top: {y}px; width: {s}px; height: {s}px; animation-delay: {delay}s; animation-duration: {dur}s;"
                    )),
            );
        }

        let like_cls = if self.liked { "like on" } else { "like" };

        html()
            .styled()
            .child(head().child(style().child(CSS)))
            .child(
                body().child(
                    div()
                        .class("app")
                        .child(div().class("pride-edge"))
                        .child(div().class("glow glow-a"))
                        .child(div().class("glow glow-b"))
                        .child(div().class("glow glow-c"))
                        .child(div().class("glow glow-d"))
                        .child(div().class("orbit-bg"))
                        .child(div().class("orbit-bg2"))
                        .child(rays(26, "big-rays", 560.0))
                        .child(div().class("halo"))
                        .child(
                            div()
                                .class("ghost-wrap")
                                .child(div().class("ghost ghost-1").child("GAYDAR"))
                                .child(div().class("ghost ghost-2").child("GAYDAR")),
                        )
                        .child(
                            div()
                                .class("bg-grain")
                                .child(canvas(|b, p| page_grain(b, p)).w(px(1440.0)).h(px(900.0))),
                        )
                        .child(particles)
                        .child(
                            div()
                                .class("shell")
                                .child(sidebar(nav_rows, barcode))
                                .child(
                                    div()
                                        .class("center")
                                        .child(topbar())
                                        .child(
                                            div()
                                                .class("hero")
                                                .child(art())
                                                .child(vinyl())
                                                .child(cert())
                                                .child(title_block())
                                                .child(phrase()),
                                        )
                                        .child(deck(wave, self.playing, self.liked, like_cls, cx))
                                        .child(notes())
                                        .child(ticker()),
                                )
                                .child(right_rail(queue, conf_segs, mood_rows)),
                        ),
                ),
            )
    }
}

fn sidebar(nav_rows: Div, barcode: Div) -> Div {
    div()
        .class("rail-left")
        .child(
            div()
                .class("brand")
                .child(
                    div()
                        .class("mark")
                        .child(div().class("mark-ring mark-r1"))
                        .child(div().class("mark-ring mark-r2"))
                        .child(div().class("mark-heart heart-clip"))
                        .child(div().class("mark-dot"))
                        .child(div().class("mark-sweep")),
                )
                .child(
                    div()
                        .class("wordmark")
                        .child(
                            div()
                                .class("wordmark-main")
                                .child("GAYDAR ")
                                .child(span().class("fm").child("FM")),
                        )
                        .child(div().class("wordmark-sub").child("QUEER RADIO / 24H")),
                ),
        )
        .child(
            div()
                .class("station-line")
                .child(div().class("bar"))
                .child(div().class("txt").child("88.7 FM · LIVE FROM THE CLOUD")),
        )
        .child(
            div()
                .class("nav-label")
                .child(div().class("mono").child("TUNE IN"))
                .child(div().class("mini").child("05 CHANNELS")),
        )
        .child(nav_rows)
        .child(
            div()
                .class("radar-box")
                .child(
                    div()
                        .class("radar-wrap")
                        .child(div().class("radar-ring rr1"))
                        .child(div().class("radar-ring rr2"))
                        .child(div().class("radar-ring rr3"))
                        .child(div().class("radar-cross-h"))
                        .child(div().class("radar-cross-v"))
                        .child(div().class("radar-sweep"))
                        .child(div().class("radar-arm"))
                        .child(div().class("blip a"))
                        .child(div().class("blip b"))
                        .child(div().class("blip c")),
                )
                .child(
                    div()
                        .class("radar-caption")
                        .child(div().class("mono").child("SCANNING FOR QUEERNESS"))
                        .child(div().class("count").child("03")),
                )
                .child(
                    div()
                        .class("mono")
                        .style("margin-top: 3px; color: #76637f; font-size: 5.8px;")
                        .child("SIGNALS NEARBY · ALL OF THEM"),
                ),
        )
        .child(div().class("spacer"))
        .child(
            div()
                .class("guest")
                .child(
                    div()
                        .class("guest-top")
                        .child(
                            div()
                                .class("guest-title")
                                .child("GUEST ")
                                .child(span().class("g2").child("DIVA")),
                        )
                        .child(div().class("guest-heart heart-clip")),
                )
                .child(div().class("guest-rule"))
                .child(div().class("guest-sub").child("NO SIGNUP. NO AUDIENCE."))
                .child(
                    div()
                        .class("guest-no")
                        .child("NO LOGIN. ")
                        .child(span().class("hl").child("EVER.")),
                )
                .child(
                    div().class("guest-foot").child(barcode).child(
                        div()
                            .class("guest-code")
                            .child("GAYDAR-001")
                            .child(div().child("ADMIT ONE · ALL NIGHT")),
                    ),
                )
                .child(div().class("guest-holo")),
        )
        .child(
            div()
                .class("alsa")
                .child(div().class("alsa-dot"))
                .child(
                    div()
                        .class("alsa-main")
                        .child(div().class("alsa-title").child("ALSA · 48 kHz"))
                        .child(div().class("alsa-sub").child("HDA INTEL PCH / hw:0,0")),
                )
                .child(
                    div()
                        .class("alsa-bars")
                        .child(div().style("height: 5px;"))
                        .child(div().style("height: 9px; animation-delay: -0.2s;"))
                        .child(div().style("height: 13px; animation-delay: -0.5s;"))
                        .child(div().style("height: 7px; animation-delay: -0.8s;")),
                ),
        )
        .child(
            div()
                .class("rail-foot")
                .child(
                    div()
                        .class("flag")
                        .child(div().class("f1"))
                        .child(div().class("f2"))
                        .child(div().class("f3"))
                        .child(div().class("f4"))
                        .child(div().class("f5"))
                        .child(div().class("f6")),
                )
                .child(
                    div()
                        .class("foot-note")
                        .child("SERVING SINCE '26")
                        .child(div().child("NO SKIPS. NO SHAME.")),
                ),
        )
}

fn topbar() -> Div {
    div()
        .class("topbar")
        .child(
            div()
                .class("crumb")
                .child(div().class("bar"))
                .child(div().class("place").child("GAYDAR FM / 88.7"))
                .child(
                    div()
                        .class("mono")
                        .style("font-size: 6px;")
                        .child("NOW PLAYING"),
                ),
        )
        .child(
            div()
                .class("live-pill")
                .child(div().class("dot"))
                .child(div().class("txt").child("ON AIR"))
                .child(div().class("time").child("23:47:08")),
        )
        .child(
            div()
                .class("top-right")
                .child(
                    div()
                        .class("serving-pill")
                        .child(div().class("txt").child("SERVING"))
                        .child(
                            div()
                                .class("mini-bars")
                                .child(div().style("height: 3px;"))
                                .child(div().style("height: 7px; animation-delay: -0.2s;"))
                                .child(div().style("height: 5px; animation-delay: -0.45s;")),
                        ),
                )
                .child(
                    div()
                        .class("top-time")
                        .child("23:47")
                        .child(span().class("tz").child(" / IST")),
                )
                .child(
                    div()
                        .class("avatar")
                        .child(div().class("letter").child("G"))
                        .child(div().class("status")),
                ),
        )
}

fn art() -> Div {
    div()
        .class("art")
        .child(div().class("art-base"))
        .child(div().class("art-glow"))
        .child(div().class("art-glow2"))
        .child(rays(22, "art-rays", 260.0))
        .child(
            div()
                .style("position: absolute; left: 0; top: 0; width: 372px; height: 372px;")
                .child(canvas(|b, p| art_texture(b, p)).w(px(372.0)).h(px(372.0))),
        )
        .child(div().class("art-orbit"))
        .child(div().class("art-orbit2"))
        .child(div().class("art-orbit-dot"))
        .child(div().class("chrome-heart-shadow heart-clip"))
        .child(div().class("heart-rim heart-clip"))
        .child(
            div().class("heart-wrap").child(
                div()
                    .class("heart-sculpt heart-clip")
                    .child(div().class("chrome-heart"))
                    .child(div().class("chrome-heart-gloss"))
                    .child(div().class("chrome-heart-spec")),
            ),
        )
        .child(div().class("art-shade"))
        .child(div().class("art-spark"))
        .child(div().class("art-shine"))
        .child(div().class("art-scan"))
        .child(div().class("art-label tl").child("GAYDAR FM"))
        .child(div().class("art-label tr").child("SIDE A · 33 RPM"))
        .child(div().class("art-rule"))
}

fn vinyl() -> Div {
    let mut spin = div().class("vinyl-spin").child(div().class("vinyl-cue"));
    for i in 0..12 {
        let d = 156.0 + i as f32 * 15.0;
        let inset = (350.0 - d) / 2.0;
        spin = spin.child(div().class("groove").style(format!(
            "left: {inset}px; top: {inset}px; width: {d}px; height: {d}px; opacity: {};",
            if i % 3 == 0 { 0.95 } else { 0.55 }
        )));
    }
    div()
        .class("vinyl")
        .child(spin)
        .child(div().class("vinyl-arc"))
        .child(div().class("vinyl-arc2"))
        .child(div().class("vinyl-tint"))
        .child(div().class("vinyl-rim"))
        .child(
            div()
                .class("vinyl-label")
                .child(div().class("label-gloss"))
                .child(div().class("l1").child("GAYDAR"))
                .child(div().class("l2").child("QUEER RADIO"))
                .child(div().class("l2").child("88.7 FM")),
        )
        .child(div().class("vinyl-glint"))
        .child(div().class("vinyl-ring"))
}

fn cert() -> Div {
    div()
        .class("cert")
        .child(div().class("cert-inner"))
        .child(div().class("c1").child("GAYDAR"))
        .child(div().class("c2").child("CERTIFIED"))
        .child(
            div()
                .class("c3")
                .child(div().class("mini-heart heart-clip"))
                .child("98.2% MATCH"),
        )
}

fn title_block() -> Div {
    div()
        .class("title-block")
        .child(
            div()
                .class("overline")
                .child(div().class("dash"))
                .child(div().class("mono").child("NOW PLAYING / TRACK 03"))
                .child(
                    div()
                        .class("mono right")
                        .style("margin-left: auto;")
                        .child("FROM THE CLOUD"),
                ),
        )
        .child(
            div()
                .class("title")
                .child(div().class("title-line").child("PINK PONY"))
                .child(
                    div()
                        .class("title-line")
                        .style("color: var(--pink);")
                        .child("CLUB"),
                ),
        )
        .child(
            div()
                .class("artist-row")
                .child(div().class("artist").child("Chappell Roan"))
                .child(div().class("artist-seal").child(div().class("check"))),
        )
        .child(
            div()
                .class("release")
                .child("THE RISE AND FALL OF A MIDWEST PRINCESS / 2023"),
        )
        .child(
            div()
                .class("stats")
                .child(
                    div()
                        .class("stat")
                        .child(div().class("k").child("LENGTH"))
                        .child(div().class("v").child("3:44")),
                )
                .child(
                    div()
                        .class("stat")
                        .child(div().class("k").child("TEMPO"))
                        .child(div().class("v").child("118 BPM")),
                )
                .child(
                    div()
                        .class("stat")
                        .child(div().class("k").child("KEY"))
                        .child(div().class("v").child("A MAJOR")),
                )
                .child(
                    div()
                        .class("stat")
                        .child(div().class("k").child("REPLAYS"))
                        .child(div().class("v pink").child("41")),
                ),
        )
}

fn phrase() -> Div {
    div()
        .class("phrase-wrap")
        .child(
            div()
                .class("phrase-label")
                .child(div().class("l"))
                .child(div().class("mono").child("TONIGHT'S READ / 88.7")),
        )
        .child(
            div()
                .class("phrase")
                .child(span().class("phrase-w").child("serving"))
                .child(span().class("phrase-w pink").child("soft-girl"))
                .child(span().class("phrase-w end").child("euphoria")),
        )
}

fn deck(
    wave: Div,
    playing: bool,
    liked: bool,
    like_cls: &'static str,
    cx: &mut Context<NowPlaying>,
) -> Div {
    div()
        .class("deck")
        .child(
            div()
                .class("deck-head")
                .child(div().class("mono").child("WAVEFORM / 24-BIT · 1411 KBPS"))
                .child(
                    div()
                        .class("mono mono-pink")
                        .child("HIGH FIDELITY, LOW SHAME"),
                ),
        )
        .child(
            div()
                .class("wave-row")
                .child(div().class("time now").child("2:19"))
                .child(wave)
                .child(div().class("time").child("-1:25")),
        )
        .child(
            div()
                .class("controls")
                .child(
                    div()
                        .class("ctrl-side")
                        .child(
                            div()
                                .class("mini-btn on")
                                .child(div().class("ic-shuffle"))
                                .child(div().class("lbl").child("SHUFFLE")),
                        )
                        .child(
                            div()
                                .class("mini-btn")
                                .child(div().class("ic-repeat"))
                                .child(div().class("lbl").child("REPEAT")),
                        ),
                )
                .child(
                    div()
                        .class("ctrl-center")
                        .child(
                            div()
                                .class("ic-prev")
                                .child(div().class("bar"))
                                .child(div().class("tri")),
                        )
                        .child(
                            div()
                                .class("play-wrap")
                                .child(div().class("play-ring"))
                                .child(
                                    div()
                                        .class("play")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.playing = !this.playing;
                                            cx.notify();
                                        }))
                                        .child(if playing {
                                            div().class("pause").child(div()).child(div())
                                        } else {
                                            div().class("tri")
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .class("ic-next")
                                .child(div().class("tri"))
                                .child(div().class("bar")),
                        ),
                )
                .child(
                    div()
                        .class("ctrl-side right")
                        .child(
                            div()
                                .class(like_cls)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.liked = !this.liked;
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .class("like-btn")
                                        .child(div().class("mini-heart heart-clip")),
                                )
                                .child(
                                    div()
                                        .class("like-meta")
                                        .child(div().class("n").child(if liked {
                                            "24.9K"
                                        } else {
                                            "24.8K"
                                        }))
                                        .child(div().class("k").child("HEARTS")),
                                ),
                        )
                        .child(
                            div()
                                .class("vol")
                                .child(div().class("k").child("VOL"))
                                .child(
                                    div()
                                        .class("vol-track")
                                        .child(div().class("vol-fill").style("width: 68%;"))
                                        .child(
                                            div().class("vol-knob").style("left: calc(68% - 4px);"),
                                        ),
                                )
                                .child(div().class("v").child("68")),
                        ),
                ),
        )
        .child(
            div()
                .class("deck-foot")
                .child(
                    div()
                        .class("deck-tags")
                        .child(div().class("tag").child("QUEER BY DESIGN"))
                        .child(div().class("tag").child("SOUNDCLOUD / AUTO-DJ"))
                        .child(div().class("tag").child("NO LOGIN REQUIRED")),
                )
                .child(
                    div()
                        .class("deck-quality")
                        .child(div().class("mono").child("STREAM / 128 KBPS"))
                        .child(div().class("hd-badge").child("HD")),
                ),
        )
}

fn notes() -> Div {
    div()
        .class("notes")
        .child(div().class("quote-mark").child("\u{201c}"))
        .child(
            div()
                .class("notes-copy")
                .child(div().class("k").child("WHY THIS TRACK"))
                .child(
                    div()
                        .class("q")
                        .child("because you keep playing songs about ")
                        .child(span().class("pink").child("leaving home"))
                        .child(", then staying."),
                ),
        )
        .child(
            div()
                .class("eq-block")
                .child(
                    div()
                        .class("eq")
                        .child(div().class("e1"))
                        .child(div().class("e2"))
                        .child(div().class("e3"))
                        .child(div().class("e4")),
                )
                .child(
                    div()
                        .class("eq-label")
                        .child(div().class("a").child("4-BAND EQ"))
                        .child(div().class("b").child("LIVE / 48 kHz")),
                ),
        )
        .child(
            div()
                .class("serve-stamp")
                .child(div().class("a").child("SERVING"))
                .child(div().class("b").child("ALSA NATIVE · BIT-PERFECT")),
        )
}

fn ticker() -> Div {
    let mut track = div().class("marquee-track");
    for _ in 0..4 {
        track = track.child(
            div()
                .class("chunk")
                .child(
                    div()
                        .class("titem")
                        .child("NO LOGIN · NO SIGNUP · NO ALGORITHM APOLOGY"),
                )
                .child(div().class("sep"))
                .child(
                    div()
                        .class("titem")
                        .child("SOUNDCLOUD RADIO FOR THE GIRLS, THEYS & THEMS"),
                )
                .child(div().class("sep"))
                .child(
                    div()
                        .class("titem")
                        .child("SERVING SOFT-GIRL EUPHORIA SINCE 2026"),
                )
                .child(div().class("sep")),
        );
    }
    div()
        .class("ticker")
        .child(div().class("tagline").child("GUEST DIVA"))
        .child(div().class("marquee").child(track))
        .child(
            div()
                .class("ticker-right")
                .child(div().class("dot"))
                .child(div().class("t").child("ALSA / 48K / SERVING")),
        )
}

fn right_rail(queue: Div, conf_segs: Div, mood_rows: Div) -> Div {
    div()
        .class("rail-right")
        .child(
            div()
                .class("rail-head")
                .child(div().class("title").child("UP NEXT"))
                .child(div().class("count").child("04 TRACKS")),
        )
        .child(
            div()
                .class("rail-head2")
                .child(div().class("mono").child("AUTO-DJ / GUEST FREQUENCY"))
                .child(div().class("mono mono-pink").child("QUEUE")),
        )
        .child(queue)
        .child(
            div()
                .class("conf")
                .child(
                    div()
                        .class("conf-head")
                        .child(div().class("lbl").child("GAYDAR CONFIDENCE"))
                        .child(div().class("live").child("LIVE CALC")),
                )
                .child(
                    div()
                        .class("conf-main")
                        .child(div().class("conf-num").child("94"))
                        .child(div().class("conf-pct").child("%"))
                        .child(div().class("conf-match").child("MATCH")),
                )
                .child(conf_segs)
                .child(
                    div()
                        .class("conf-note")
                        .child("you keep playing songs about ")
                        .child(span().class("pink").child("leaving home"))
                        .child(" and dancing anyway."),
                ),
        )
        .child(
            div()
                .class("mood")
                .child(
                    div()
                        .class("mood-head")
                        .child(div().class("lbl").child("MOOD DIALS"))
                        .child(div().class("auto").child("AUTO-TUNED")),
                )
                .child(mood_rows),
        )
        .child(
            div()
                .class("stream")
                .child(
                    div()
                        .class("stream-head")
                        .child(div().class("lbl").child("STREAM"))
                        .child(div().class("val").child("SOUNDCLOUD · HQ")),
                )
                .child(div().class("stream-wave").children((0..22).map(|i| {
                    let h = 5.0 + ((i * 7 + 3) % 19) as f32;
                    let color = match i % 3 {
                        0 => "#ff4fa3",
                        1 => "#c7a6ff",
                        _ => "#59e6ff",
                    };
                    div().style(format!(
                        "height: {h}px; background: {color}; opacity: 0.55;"
                    ))
                })))
                .child(
                    div()
                        .class("stream-foot")
                        .child(div().class("mono").child("NO ACCOUNT REQUIRED"))
                        .child(div().class("mono mono-pink").child("AUTO-DJ ON")),
                ),
        )
        .child(div().class("rail-space"))
        .child(
            div()
                .class("rail-foot")
                .child(
                    div()
                        .class("signal")
                        .child(div().class("lbl").child("SIGNAL"))
                        .child(
                            div()
                                .class("signal-bars")
                                .child(div().style("height: 4px;"))
                                .child(div().style("height: 6px;"))
                                .child(div().style("height: 8px;"))
                                .child(div().style("height: 10px;"))
                                .child(div().style("height: 7px; opacity: 0.35;")),
                        )
                        .child(div().class("val").child("EXCELLENT")),
                )
                .child(
                    div()
                        .class("rail-foot-bottom")
                        .child(
                            div()
                                .class("fl")
                                .child(div().style("background: #ff4fa3;"))
                                .child(div().style("background: #ff8a5c;"))
                                .child(div().style("background: #ffe066;"))
                                .child(div().style("background: #b6ff5c;"))
                                .child(div().style("background: #59e6ff;"))
                                .child(div().style("background: #c7a6ff;")),
                        )
                        .child(
                            div()
                                .class("note")
                                .child("MADE FOR THE GIRLS,")
                                .child(div().child("THEYS & THEMS"))
                                .child(div().class("ver").child("GAYDAR FM · 88.7")),
                        ),
                ),
        )
}

fn save_png(path: &str, w: usize, h: usize, px: &[u32]) {
    let rgba: Vec<u8> = px
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
        .collect();
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgba)
        .expect("png data");
    println!("wrote {path} ({w}x{h})");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let at_ms = args
        .iter()
        .position(|a| a == "--at")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(2600);
    let app = if shot.is_some() {
        Application::headless()
    } else {
        Application::new()
    };
    app.run(move |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(1440.0), px(900.0)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("GAYDAR FM - Now Playing".into()),
            }),
            ..Default::default()
        };
        let win = cx
            .open_window(opts, |_, cx| {
                cx.new(|_| NowPlaying {
                    playing: true,
                    liked: true,
                    nav: 0,
                })
            })
            .unwrap();
        if let Some(path) = shot {
            cx.draw(win.id);
            cx.advance_clock(Duration::from_millis(at_ms));
            cx.draw(win.id);
            let (w, h, px) = cx.draw(win.id);
            save_png(&path, w, h, &px);
        }
    });
}
