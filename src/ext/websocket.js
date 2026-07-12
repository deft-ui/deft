import * as wsConnectionApi from "native";
export class WebSocket {

    client;

    listeners;

    onopen;

    onclose;

    onmessage;

    onping;

    onpong;

    onerror;

    #closed = false;

    constructor(url) {
        this.listeners = Object.create(null);
        this.#connect(url);
    }

    addEventListener(name, callback) {
        if (!this.listeners[name]) {
            this.listeners[name] = [];
        }
        const listeners = this.listeners[name]
        listeners.push(callback);
    }

    async send(data) {
        try {
            await wsConnectionApi.sendStr(this.client, data + "");
        } catch (error) {
            this.#emit('error', error);
        }
    }

    close() {
        if (!this.#closed) {
            this.#closed = true;
            this.#emit("close");
            wsConnectionApi.close(this.client);
        }
    }

    async #connect(url) {
        try {
            this.client = await wsConnectionApi.connect(url);
            this.#emit("open");
            this.#doRead();
        } catch (error) {
            this.#emit("error", error);
        }

    }

    async #doRead() {
        try {
            loop:
                for (;;) {
                    let [type, data] = await wsConnectionApi.read(this.client);
                    // console.log("read message", type, data);
                    switch (type) {
                        case "text":
                            this.#emit("message", data);
                            break;
                        case "binary":
                            this.#emit("message", ArrayBuffer.from(data));
                            break;
                        case "ping":
                            this.#emit("ping", data);
                            break;
                        case "pong":
                            this.#emit("pong", data);
                            break;
                        case "close":
                            break loop;
                        case "frame":
                            this.#emit("frame", data);
                            break;
                    }
                }
            //TODO maybe half-close?
            this.close();
        } catch (error) {
            console.error(error);
            this.#emit("error");
            this.close();
        }
    }

    #emit(name, data) {
        // console.log("emit", name, data);
        /**
         * @type {Event}
         */
        let event = {
            bubbles: false,
            cancelBubble: false,
            cancelable: false,
            composed: false,
            currentTarget: null,
            eventPhase: 0,
            isTrusted: true,
            returnValue: false,
            srcElement: null,
            target: null,
            timeStamp: new Date().getTime(),
            type: name,
            data,
        };
        const key = `on${name}`;
        if (this[key]) {
            try {
                this[key](event)
            } catch (error) {
                console.error(error);
            }
        }
        for (const listener of this.listeners[name] || []) {
            try {
                listener(event);
            } catch (error) {
                console.error(error);
            }
        }
    }

}

globalThis.WebSocket = WebSocket;