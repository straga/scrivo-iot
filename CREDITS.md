# Credits

## Code taken into this tree

Each of these is ours to change now; the source and the terms are kept here.

| What | Where | From | License |
|---|---|---|---|
| mDNS component | `crates/firmware/mdns/` | [espressif/mdns](https://github.com/espressif/esp-protocols) 1.3.2 (`components/mdns` of esp-protocols, commit `5d69d3f0ac34da638e8cb99eacb19d5b862afa8b`); examples, tests and `idf_component.yml` left out; the change we made is listed in `TAKEN_FROM.md` there | Apache-2.0, `crates/firmware/mdns/LICENSE` |
| OWL 2.8.3 | `page/owl.iife.min.js` | [odoo/owl](https://github.com/odoo/owl), npm package `@odoo/owl`, `dist/owl.iife.min.js`; the component framework of the page, kept in the tree so the page is served from the board without fetching anything | LGPL-3.0-only |

## Used to build, not shipped as source

- [Tailwind CSS](https://tailwindcss.com) 3.4.17 (MIT) generates `page/app.tw.css` (`tools/page_css.py`).
- [ESP-IDF](https://github.com/espressif/esp-idf) 5.5.2 (Apache-2.0) and the Rust ESP ecosystem (`esp-idf-svc`,
  `esp-idf-sys`, `espup`, `ldproxy`) are what the images are built on.
- [Embassy](https://embassy.dev) (`embassy-sync`, `embassy-time`), [edge-net](https://github.com/ivmarkov/edge-net)
  (`edge-http`, `edge-ws`, `edge-nal`) and the other crates named in `Cargo.lock`, each under its own license.

## Companion project

[scrivo-safe](https://github.com/straga/scrivo-safe), the recovery image these firmwares are installed through (MIT).

## Who built it

Built by **straga** together with AI coding agents: **Claude Code** (Anthropic), **Codex** (OpenAI), **GLM** (Z.ai)
and a local **Qwen** model. The agents wrote much of the code, tests and this documentation; every change was reviewed and
tested on real boards before it went in.
