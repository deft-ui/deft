import * as fetchApi from "native";

class FetchResponse {
    _resp;

    constructor(resp, status) {
        this._resp = resp;
        this.status = status;
        this.ok = this.status >= 200 && this.status < 300;
    }

    async json() {
        const body = await fetchApi.responseBodyString(this._resp);
        return JSON.parse(body);
    }

}

/**
 *
 * @param url {string}
 * @param options {FetchOptions}
 * @returns {Promise<FetchResponse>}
 */
export async function fetch(url, options) {
    const resp = await fetchApi.create(url, options);
    let status = await fetchApi.responseStatus(resp);
    return new FetchResponse(resp, status);
}


globalThis.fetch = fetch;