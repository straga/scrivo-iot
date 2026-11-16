// Where the files of a device's page are read from by the tests that load them: the engine's page (page/) has the
// engine's files, the device's folder (devices/<device>/page/) has what the device adds - so a file is looked for in the
// engine first and in the device's folder after it, and a test says WHICH file it wants, never in which folder it lies.
//
// The menu and the table of texts are one file each on the board, merged from the two halves by tools/page_kit.py; the
// merge is done THERE and only there, and a test asks for the result (`menu`, `texts`) instead of merging by itself - a
// second merge written here would be a copy that goes its own way.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const ROOT = path.resolve(__dirname, "..");

// the path of FILE of DEVICE's page: the engine's when the engine has it, the device's otherwise
function source(file, device = "burner") {
    for (const folder of [path.join(ROOT, "page"), path.join(ROOT, "devices", device, "page")]) {
        const at = path.join(folder, file);
        if (fs.existsSync(at)) return at;
    }
    throw new Error(`no ${file} in the engine's page or in the page folder of ${device}`);
}

function read(file, device = "burner") {
    return fs.readFileSync(source(file, device), "utf8");
}

// FILES (of the page) run in SCOPE as the page's own scripts do: one after the other, each given `globalThis`
function load(files, scope, device = "burner") {
    for (const file of files) new Function("globalThis", read(file, device))(scope);
}

// the merge is asked of Python once for a flag and a device: a test file reads the menu in several places, and a copy it may change is its own
const asked = new Map();
const merged = (flag, device) => {
    const key = `${flag} ${device}`;
    if (!asked.has(key)) {
        asked.set(key, execFileSync("uv", ["run", "--no-project", "python", path.join(__dirname, "page_sources.py"), flag, device], { cwd: ROOT, encoding: "utf8", maxBuffer: 64 << 20 }));
    }
    return JSON.parse(asked.get(key));
};

const menu = (device = "burner") => merged("--menu", device);
const texts = (device = "burner") => merged("--texts", device);

module.exports = { ROOT, source, read, load, menu, texts };
