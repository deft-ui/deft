import * as clipboardApi from "native";

export class Clipboard {
    /**
     *
     * @returns {Promise<string>}
     */
    async readText() {
        return clipboardApi.readText();
    }

    /**
     *
     * @param text {string}
     * @returns {Promise<void>}
     */
    async writeText(text) {
        clipboardApi.writeText(text);
    }
}

navigator.clipboard = new Clipboard();