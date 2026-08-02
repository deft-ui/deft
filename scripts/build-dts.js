const fs = require("node:fs");
const {spawnSync} = require("node:child_process");
function item(dir, name, namespace = "") {
    return {
        src: `${dir}/${name}.js`,
        namespace,
    }
}
const list = [
    item("src/js", "core", "deft:core"),
    item("src/js", "ui", "deft:ui"),
    item("src/js", "menu", "deft:menu"),
    item("src/ext", "system-tray", "deft:systemtray"),
    item("src/ext", "process", "deft:process"),
    item("src/ext", "audio", "deft:audio"),
    item("src/ext", "clipboard", "deft:clipboard"),
    item("src/ext", "dialog", "deft:dialog"),
    item("src/ext", "stylesheet", "deft:core:stylesheet"),
    item("src/ext", "jsapp", "deft:core:jsapp"),
    item("src/ext", "sqlite", "deft:sqlite"),
    item("src/ext", "fetch")
]

const paths = Object.create(null);
for (const it of list) {
    if (it.namespace) {
        paths[it.namespace] = [it.src];
    }
}
const files = list.map(it => it.src);

const config = {
    "compilerOptions": {
        "target": "esnext",
        "lib": ["esnext"],
        "declaration": true,
        "allowJs": true,
        "emitDeclarationOnly": true,
        "outDir": "target/deft-dts",
        "baseUrl": ".",
        paths,
    },
    files,
};

fs.rmSync("target/deft-dts", {recursive: true, force: true});
fs.writeFileSync("tsconfig.dts.json", JSON.stringify(config), {encoding: "utf-8"});
spawnSync(`npx`, ["-p", "typescript@5", "tsc", "-p", "tsconfig.dts.json"], {
    stdio: "inherit",
});

function outputPath(src) {
    return `target/deft-dts/${src.replace(/^src\//, "").replace(/\.js$/, ".d.ts")}`;
}

for (const it of list) {
    const file = outputPath(it.src);
    let content = fs.readFileSync(file, {encoding: "utf-8"});
    if (it.namespace) {
        content = content.replace(/export declare /g, "export ").replace(/#private;/g, "");
        content = `declare module '${it.namespace}' {\n${content}}\n`;
    } else {
        content = content.replace(/export /g, "declare ").replace(/#private;/g, "").replace(/declare {};/g, "");
    }
    fs.writeFileSync(file, content, {encoding: "utf-8"});
}

const parts = list.map(it => outputPath(it.src));
fs.writeFileSync(
    "js/deft.d.ts",
    [
        fs.readFileSync("env.d.ts", "utf-8"),
        ...parts.map(f => fs.readFileSync(f, "utf-8"))
    ].join("\n"), {encoding: "utf-8"}
);



