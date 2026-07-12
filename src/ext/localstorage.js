import * as localStorageApi from "native";

/**
 *
 * @type {LocalStorage}
 */
const localStorage = {
    getItem(key) {
        return localStorageApi.get(key)
    },
    setItem(key, value) {
        localStorageApi.set(key, value);
    }
}
globalThis.localStorage = localStorage;