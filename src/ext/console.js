import * as consoleApi from "native";

function collectCircleRefInfo(value, visited, circleRefList, level) {
    if (level >= 3) {
        return;
    }
    if (value && typeof value === "object") {
        if (visited.includes(value)) {
            circleRefList.push(value);
            return;
        } else {
            visited.push(value);
        }
        Object.entries(value).forEach(([k, v]) => {
            collectCircleRefInfo(v, visited, circleRefList, level + 1);
        })
    }
}

function log(...values) {
    values.forEach((value, index) => {
        const visited = [];
        const circleRefList = [];
        collectCircleRefInfo(value, visited, circleRefList, 0);
        printObj(value, "", circleRefList, [], 0);
        if (index < values.length - 1) {
            printObj(",")
        }
    })
    consoleApi.print("\n");
}

function printObj(value, padding, circleRefList, printedList, level) {
    let type = typeof value;
    if (value instanceof Error) {
        console.log(`[Error(${value.name})]` + value.message);
        if (value.stack) {
            console.log(value.stack);
        }
    } else if (type === "object" && value != null) {
        const refIdx = circleRefList.indexOf(value);
        if (refIdx >= 0 && printedList.includes(value)) {
            consoleApi.print("[Circular *" + refIdx + "]");
        } else {
            const entries = Object.entries(value);
            if (level >= 2) {
                return "{...}"
            }
            if (!entries.length) {
                consoleApi.print("{}");
            } else {
                const prefix = refIdx >= 0 ? ("<ref *" + refIdx + ">") : "";
                consoleApi.print(prefix + "{\n");
                printedList.push(value);
                entries.forEach(([k, v], index) => {
                    consoleApi.print(padding + "  " + k + ":");
                    printObj(v, padding + "  ", circleRefList, printedList, level + 1);
                    if (index < entries.length - 1) {
                        consoleApi.print(",\n");
                    }
                });
                consoleApi.print("\n" + padding + "}");
            }
        }
    } else if (type === "symbol") {
        console.log("[Symbol]")
    } else if (type === "function") {
        console.log("[Function]")
    } else {
        consoleApi.print(value + "");
    }
}

globalThis.console = {
    trace: log,
    debug: log,
    log,
    info: log,
    warn: log,
    error: log,
}