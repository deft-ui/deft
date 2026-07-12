import * as processApi from "native";
export class Process {
    /**
     *
     * @param code {number}
     */
    exit(code) {
        processApi.exit(code);
    }

    /**
     *
     * @param value {boolean}
     */
    setExitOnAllWindowsClosed(value) {
        processApi.setExitOnAllWindowsClosed(value);
    }

    /**
     *
     * @returns {string[]}
     */
    get argv() {
        return processApi.argv();
    }

    /**
     *
     * @returns {boolean}
     */
    get isMobilePlatform() {
        return processApi.isMobilePlatform();
    }

    /**
     *
     * @returns {string}
     */
    get platform() {
        return processApi.platform();
    }

    /**
     *
     * @param handler {Function}
     */
    setPromiseRejectionTracker(handler) {
        processApi.setPromiseRejectionTracker(handler);
    }
}

globalThis.process = new Process();
globalThis.process.setPromiseRejectionTracker(error => {
    console.error('uncaught promise error', error);
});