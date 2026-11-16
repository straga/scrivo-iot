// The page's own tokens carried into Tailwind by name: a class says bg-panel or text-dim, and the value comes
// from the CSS variable - so the dark theme keeps working without a second set of classes.
//
// Read by tools/page_css.py, which runs Tailwind from this folder: content is the engine's page and the scripts of
// every device's page folder, and what comes out is page/app.tw.css, the one stylesheet of every device. Only the
// classes those files use are written, so the page and its stylesheet have to be built together.
module.exports = {
  content: ["../index.html", "../page.js", "../../devices/*/page/*.js"],
  theme: {
    extend: {
      colors: {
        bg: "var(--bg)", panel: "var(--panel)", sunk: "var(--sunk)", line: "var(--line)",
        ink: "var(--ink)", dim: "var(--dim)", acc: "var(--acc)", "acc-ink": "var(--acc-ink)",
        "acc-soft": "var(--acc-soft)", warn: "var(--warn)", ok: "var(--ok)", bad: "var(--bad)",
        "flow-hot": "var(--flow-hot)", "flow-cold": "var(--flow-cold)",
      },
      fontFamily: { sans: "var(--sans)", mono: "var(--mono)" },
      borderColor: { DEFAULT: "var(--line)" },
      // A card's shadow is a token too, because the light theme needs one and the dark theme must not have it:
      // a shadow on a dark page is a smudge. tokens.css sets it per theme.
      boxShadow: { card: "var(--shadow-card)" },
    },
  },
};
