/**
 * @template D
 */
export class EventObject {
    _propagationCancelled = false
    _preventDefault = false
    type;
    /**
     * @type {D}
     */
    detail;

    constructor(type, detail) {
        this.type = type;
        this.detail = detail;
    }

    stopPropagation() {
        this._propagationCancelled = true;
    }

    preventDefault() {
        this._preventDefault = true;
    }

    result() {
        return {
            propagationCancelled: this._propagationCancelled,
            preventDefault: this._preventDefault,
        }
    }

}

export class EventBinder {
    #eventListeners = Object.create(null);
    #target;
    #removeEventListenerApi;
    #addEventListenerApi;
    #self;
    #allEventListeners = Object.create(null);

    constructor(target, addApi, removeApi, self) {
        this.#target = target;
        this.#addEventListenerApi = addApi;
        this.#removeEventListenerApi = removeApi;
        this.#self = self;
    }

    bindEvent(type, callback) {
        type = type.toLowerCase();
        if (typeof callback !== "function") {
            throw new Error("invalid callback");
        }
        let oldListener = this.#eventListeners[type];
        if (oldListener) {
            this.removeEventListener(type, oldListener);
        }
        this.addEventListener(type, callback);
        this.#eventListeners[type] = callback;
    }
    addEventListener(type, callback) {
        const self = this.#self;

        /**
         *
         * @param detail {object}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(detail) {
            const event = new EventObject(type, detail);
            try {
                callback && callback(event);
            } catch (error) {
                console.error(`${type} event handling error, detail=`, detail ,error.message || error);
            }
            return event.result();
        }
        if (!this.#allEventListeners[type]) {
            this.#allEventListeners[type] = new Map();
        }
        const id = this.#target ? this.#addEventListenerApi(this.#target, type, eventCallback) : this.#addEventListenerApi(type, eventCallback);
        this.#allEventListeners[type].set(callback, id);
        return id;
    }

    removeEventListener(type, callback) {
        /**
         * @type {Map}
         */
        const map = this.#allEventListeners[type];
        const id = map.get(callback);
        if (id) {
            map.delete(callback);
            this.#target ? this.#removeEventListenerApi(this.#target, id) : this.#removeEventListenerApi(id);
        }
    }

}

export class EventRegistry {
    eventListeners = Object.create(null);
    _id;
    _remove_api;
    _add_api;
    #self;

    constructor(id, addApi, removeApi, self) {
        this._id = id;
        this._add_api = addApi;
        this._remove_api = removeApi;
        this.#self = self;
    }

    bindEvent(type, callback) {
        type = type.toLowerCase();
        if (typeof callback !== "function") {
            throw new Error("invalid callback");
        }
        let oldListenerId = this.eventListeners[type];
        if (oldListenerId) {
            this._remove_api(this._id, type, oldListenerId);
        }

        const self = this.#self;

        /**
         *
         * @param type {string}
         * @param detail {object}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(type, detail) {
            const event = new EventObject(type, detail);
            try {
                callback && callback(event);
            } catch (error) {
                console.error(`${type} event handling error, detail=`, detail ,error.message || error);
            }
            return event.result();
        }

        this.eventListeners[type] = this._add_api(this._id, type, eventCallback);
    }
}

globalThis.navigator = {};

globalThis.KEY_MOD_CTRL = 0x1;
globalThis.KEY_MOD_ALT = 0x1 << 1;
globalThis.KEY_MOD_META = 0x1 << 2;
globalThis.KEY_MOD_SHIFT = 0x1 << 3;
