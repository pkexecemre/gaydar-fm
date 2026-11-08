(() => {
  const dials = {
    camp: { el: document.getElementById("camp"), word: "cabaret" },
    club: { el: document.getElementById("club"), word: "ballroom" },
    soft: { el: document.getElementById("soft"), word: "ballads" },
    chaos: { el: document.getElementById("chaos"), word: "hyperpop" },
  };
  const reading = document.getElementById("reading");
  const presets = Array.from(document.querySelectorAll("[data-preset]"));

  const valueOf = (name) => Number(dials[name].el.value);

  const fill = (el) => {
    el.style.setProperty("--fill", `${(Number(el.value) * 100).toFixed(1)}%`);
  };

  const say = (name, value) => {
    const out = dials[name].el.parentElement.querySelector(".dial__value");
    if (out) out.textContent = value.toFixed(2);
    dials[name].el.setAttribute(
      "aria-valuetext",
      `${name}, ${value.toFixed(2)} of 1.00, ${dials[name].el.parentElement.querySelector(".dial__note").textContent}`
    );
  };

  const read = () => {
    const ranked = Object.keys(dials).sort((a, b) => valueOf(b) - valueOf(a));
    const first = dials[ranked[0]].word;
    const second = dials[ranked[1]].word;
    reading.textContent = `The girls are getting ${first} with a side of ${second}.`;

    const signature = Object.keys(dials).map((k) => valueOf(k).toFixed(2)).join(",");
    for (const chip of presets) {
      chip.setAttribute("aria-pressed", String(chip.dataset.preset === signature));
    }
  };

  for (const [name, dial] of Object.entries(dials)) {
    fill(dial.el);
    say(name, valueOf(name));
    dial.el.addEventListener("input", () => {
      fill(dial.el);
      say(name, valueOf(name));
      read();
    });
  }

  for (const chip of presets) {
    chip.addEventListener("click", () => {
      const values = chip.dataset.preset.split(",").map(Number);
      Object.keys(dials).forEach((name, i) => {
        dials[name].el.value = values[i].toFixed(2);
        fill(dials[name].el);
        say(name, values[i]);
      });
      read();
    });
  }

  read();

  const copy = document.getElementById("copy");
  if (copy) {
    copy.addEventListener("click", async () => {
      const text = document.getElementById("cmd").textContent.trim();
      try {
        await navigator.clipboard.writeText(text);
      } catch (error) {
        const range = document.createRange();
        range.selectNodeContents(document.getElementById("cmd"));
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        document.execCommand("copy");
        selection.removeAllRanges();
      }
      copy.dataset.copied = "true";
      copy.textContent = "Copied";
      window.setTimeout(() => {
        copy.dataset.copied = "false";
        copy.textContent = "Copy";
      }, 1800);
    });
  }
})();
