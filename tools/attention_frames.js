// What the card of what asks for a person would hold at every second of a run: the page's own pipeline - the dictionary's rows made of the board's values, then `scheme.attention` - run over the frames a scenario makes, so a failure the card
// names where nothing is wrong is found on every second of a run and not on the one the browser was asked at.
//
//     node tools/attention_frames.js INPUT.json     INPUT: {"dictionary": ..., "frames": [{"at", "values": [[address, value]]}]}
//
// Prints a JSON list, one entry for each second: {"at", "doing", "keys": [what asks], "warn": [the zones whose mark is in the warning colour]}. The settings are the dictionary's defaults and what INPUT's `settings` says over them.
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu: mergedMenu } = require("./page_sources");
const scope = {};
load(["wire.js", "dictionary.js", "scheme.js"], scope);
const { dictionary, scheme } = scope;

const input = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const menu = mergedMenu();
const texts = { menu, lang: "en" };
dictionary.useUnits(dictionary.menuUnits(texts));
const words = dictionary.schemeWords(texts);

const out = input.frames.map((frame) => {
    const values = new Array(input.dictionary.things.length).fill(undefined);
    for (const [at, value] of frame.values) values[at] = value;
    const things = dictionary.state(input.dictionary, texts, values).things;
    const doing = things.find((t) => t.group === "automaton" && t.key === "doing");
    const settings = input.dictionary.settings.map((one) => ({ name: one.name, value: input.settings && input.settings[one.name] !== undefined ? input.settings[one.name] : one.default }));
    const zones = scheme.zonesOf(things, settings);
    return { at: frame.at, doing: (doing && doing.value && doing.value.word) || null, keys: scheme.attention(things, words, settings).map((i) => i.key), warn: Object.keys(zones).filter((name) => zones[name] && zones[name].warn) };
});
process.stdout.write(JSON.stringify(out));
