// What a device adds to the engine's page - a tab, a part for a section of the menu - and what the page makes of an adding that is wrong. It lives apart
// from the components so that it is tested on its own (tools/test_page_registry.js).
//
// A WRONG ADDING IS A PROBLEM THE PAGE SAYS, NEVER A CHOICE THE ORDER OF THE SCRIPTS MAKES. A part or a tab added twice under one name would be won by
// whichever script loaded last, and a section that asks for a part nobody added would be drawn as a plain one with the person none the wiser: both are
// kept here as `problems`, in codes the page puts into words of the menu's `ui` (page.js, Page.problems), and the one that came first stands.
//
// A SECTION ASKS FOR A PART BY ITS `part` FIELD AND BY NOTHING ELSE: a key of the section that is spelled like the name of a part asks for nothing.
// What it asks the part for stands in its `given` field, and the part is given it as its prop `given`.
(function (root) {
    "use strict";

    function make() {
        const tabs = [];
        const sections = {};
        const problems = [];
        return {
            tabs,
            sections,
            problems,
            tab(spec) {
                if (tabs.some((one) => one.id === spec.id)) {
                    problems.push({ kind: "tabTwice", key: spec.id });
                    return;
                }
                tabs.push(spec);
            },
            section(key, component) {
                if (Object.prototype.hasOwnProperty.call(sections, key)) {
                    problems.push({ kind: "sectionTwice", key });
                    return;
                }
                sections[key] = component;
            },
            // null for a plain section; {component, given} for one whose part is added; {problem} for one that asks for a part nobody added
            partOf(section) {
                if (section.part === undefined) return null;
                if (!Object.prototype.hasOwnProperty.call(sections, section.part)) {
                    return { problem: { kind: "noPart", key: section.part, section: section.id } };
                }
                return { component: sections[section.part], given: section.given };
            },
        };
    }

    root.registry = { make };
})(typeof globalThis !== "undefined" ? globalThis : this);
