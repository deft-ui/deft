const VT_CONTAINER = 1
const VT_LABEL = 2
const VT_BUTTON = 3
const VT_ENTRY = 4
const VT_GROUP = 5
const VT_PROGRESS_BAR = 6
const VT_SCROLL = 7
const VT_TEXT_EDIT = 8
const VT_IMAGE = 9;

export class Frame {

    /**
     * @type EventRegistry
     */
    eventRegistry;

    frameId;

    constructor(attrs) {
        this.frameId = frame_create(attrs || {});
        this.eventRegistry = new EventRegistry(this.frameId, frame_bind_event, frame_remove_event_listener);
    }

    /**
     *
     * @param view {View}
     */
    setBody(view) {
        frame_set_body(this.frameId, view.el);
    }

    /**
     *
     * @param title {string}
     */
    setTitle(title) {
        frame_set_title(this.frameId, title);
    }

    resize(size) {
        frame_resize(this.frameId, size);
    }

    /**
     *
     * @param owner {Frame}
     */
    setModal(owner) {
        frame_set_modal(this.frameId, owner.frameId)
    }

    close() {
        frame_close(this.frameId);
    }

    /**
     *
     * @param visible {boolean}
     */
    setVisible(visible) {
        frame_set_visible(this.frameId, visible);
    }

    bindClose(callback) {
        this.bindEvent("close", callback);
    }

    bindEvent(type, callback) {
        this.eventRegistry.bindEvent(type, callback);
    }

}

export class EventObject {
    _propagationCancelled = false
    _preventDefault = false
    type;
    detail;
    target;

    constructor(type, detail, target) {
        this.type = type;
        this.detail = detail;
        this.target = target;
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

export class EventRegistry {
    eventListeners = Object.create(null);
    _id;
    _remove_api;
    _add_api;

    constructor(id, addApi, removeApi) {
        this._id = id;
        this._add_api = addApi;
        this._remove_api = removeApi;
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

        /**
         *
         * @param type {string}
         * @param detail {object}
         * @param target {unknown}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(type, detail, target) {
            const event = new EventObject(type, detail, target);
            callback && callback(event);
            return event.result();
        }

        this.eventListeners[type] = this._add_api(this._id, type, eventCallback);
    }
}

export class SystemTray {
    /**
     * @type EventRegistry
     */
    eventRegistry;
    tray;
    constructor() {
        this.tray = tray_create("Test");
        this.eventRegistry = new EventRegistry(this.tray, tray_bind_event, tray_remove_event_listener);
    }

    setTitle(title) {
        tray_set_title(this.tray, title);
    }

    setIcon(icon) {
        tray_set_icon(this.tray, icon);
    }

    setMenus(menus) {
        tray_set_menus(this.tray, menus);
    }

    bindActivate(callback) {
        this.eventRegistry.bindEvent("activate", callback);
    }

    bindMenuClick(callback) {
        this.eventRegistry.bindEvent("menuclick", callback);
    }

}
export class View {
    /**
     * @type {ContainerBasedElement}
     */
    parent
    /**
     * @type number
     */
    el

    viewType

    /**
     * @type EventRegistry
     */
    eventRegistry;

    /**
     *
     * @param viewType {number}
     */
    constructor(viewType) {
        this.viewType = viewType;
        this.el = view_create(viewType);
        if (!this.el) {
            throw new Error("Failed to create view:" + viewType)
        }
        this.eventRegistry = new EventRegistry(this.el, view_bind_event, view_remove_event_listener);
    }

    /**
     *
     * @param style {Record<string, any>}
     */
    setStyle(style) {
        view_set_style(this.el, style);
    }

    /**
     *
     * @param style {Record<string, any>}
     */
    setHoverStyle(style) {
        view_set_hover_style(this.el, style);
    }

    /**
     *
     * @param value {number}
     */
    setScrollTop(value) {
        view_set_property(this.el, "scrollTop", value);
    }

    /**
     *
     * @param value {number}
     */
    setScrollLeft(value) {
        view_set_property(this.el, "scrollLeft", value);
    }


    /**
     *
     * @param value {boolean}
     */
    setDraggable(value) {
        view_set_property(this.el, "draggable", value);
    }

    /**
     *
     * @param value {string}
     */
    setCursor(value) {
        view_set_property(this.el, "cursor", value);
    }

    /**
     *
     * @returns {[number, number]}
     */
    getSize() {
        return view_get_property(this.el, "size");
    }

    getContentSize() {
        return view_get_property(this.el, "content_size");
    }

    bindFocus(callback) {
        this.bindEvent("focus", callback);
    }

    bindBlur(callback) {
        this.bindEvent("blur", callback);
    }

    bindClick(callback) {
        this.bindEvent("click", callback);
    }

    bindMouseDown(callback) {
        this.bindEvent("mousedown", callback);
    }

    bindMouseUp(callback) {
        this.bindEvent("mouseup", callback);
    }

    bindMouseMove(callback) {
        this.bindEvent("mousemove", callback);
    }

    bindMouseEnter(callback) {
        this.bindEvent("mouseenter", callback);
    }

    bindMouseLeave(callback) {
        this.bindEvent("mouseleave", callback);
    }

    bindKeyDown(callback) {
        this.bindEvent("keydown", callback);
    }

    bindKeyUp(callback) {
        this.bindEvent("keyup", callback);
    }

    bindSizeChanged(callback) {
        this.bindEvent("sizechange", callback);
    }

    bindScroll(callback) {
        this.bindEvent("scroll", callback);
    }

    bindMouseWheel(callback) {
        this.bindEvent("mousewheel", callback);
    }

    bindDragStart(callback) {
        this.bindEvent("dragstart", callback);
    }

    bindDragOver(callback) {
        this.bindEvent("dragover", callback);
    }

    bindDrop(callback) {
        this.bindEvent("drop", callback);
    }

    bindEvent(type, callback) {
        this.eventRegistry.bindEvent(type, callback);
    }

    toString() {
        return this.el + "@" + this.viewType
    }

}

export class Audio {
    context;
    eventRegistry;
    id;
    constructor(config) {
        this.id = audio_create(config || {})
        this.eventRegistry = new EventRegistry(this.id, audio_add_event_listener, audio_remove_event_listener);
    }

    play() {
        audio_play(this.id);
    }

    pause() {
        audio_pause(this.id);
    }

    stop() {
        audio_stop(this.id);
    }

    bindLoad(callback) {
        this.eventRegistry.bindEvent('load', callback);
    }

    bindTimeUpdate(callback) {
        this.eventRegistry.bindEvent("timeupdate", callback);
    }

    bindEnd(callback) {
        this.eventRegistry.bindEvent("end", callback);
    }

    bindPause(callback) {
        this.eventRegistry.bindEvent("pause", callback);
    }

    bindStop(callback) {
        this.eventRegistry.bindEvent("stop", callback);
    }

    bindCurrentChange(callback) {
        this.eventRegistry.bindEvent("currentchange", callback);
    }

    bindEvent(type, callback) {
        this.eventRegistry.bindEvent(type, callback);
    }

}

export class LabelElement extends View {
    constructor() {
        super(VT_LABEL);
    }

    /**
     *
     * @param text {string}
     */
    setText(text) {
        view_set_property(this.el, "text", text);
    }

    /**
     *
     * @param align {"left" | "right" | "center"}
     */
    setAlign(align) {
        view_set_property(this.el, "align", align);
    }

}

export class ImageElement extends View {
    constructor() {
        super(VT_IMAGE);
    }
    setSrc(src) {
        view_set_property(this.el, "src", src);
    }
}

export class ButtonElement extends View {
    constructor() {
        super(VT_BUTTON);
    }

    /**
     *
     * @param title {string}
     */
    setTitle(title) {
        view_set_property(this.el, "title", title);
    }

}

export class EntryElement extends View {
    constructor() {
        super(VT_ENTRY);
    }

    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    setAlign(align) {
        view_set_property(this.el, "align", align);
    }

    /**
     *
     * @param text {string}
     */
    setText(text) {
        view_set_property(this.el, "text", text);
    }

    /**
     *
     * @param multipleLine {boolean}
     */
    setMultipleLine(multipleLine) {
        view_set_property(this.el, "multipleline", String(multipleLine));
    }

    /**
     *
     * @returns {string}
     */
    getText() {
        return view_get_property(this.el, "text");
    }

    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

}

class TextEditElement extends View {
    constructor() {
        super(VT_TEXT_EDIT);
    }

    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    setAlign(align) {
        view_set_property(this.el, "align", align);
    }

    /**
     *
     * @param text {string}
     */
    setText(text) {
        view_set_property(this.el, "text", text);
    }

    /**
     *
     * @returns {string}
     */
    getText() {
        return view_get_property(this.el, "text");
    }

    /**
     *
     * @param selection {[number, number]}
     */
    setSelection(selection) {
        view_set_property(this.el, "selection", selection);
    }

    /**
     *
     * @param caret {number}
     */
    setCaret(caret) {
        view_set_property(this.el, "caret", caret);
    }

    /**
     *
     * @param top {number}
     */
    scrollToTop(top) {
        view_set_property(this.el, "scroll_to_top", top);
    }

    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

    bindCaretChange(callback) {
        this.bindEvent("caretchange", callback);
    }

}

class ContainerBasedElement extends View {
    #children = [];
    
    /**
     *
     * @param child {View}
     * @param index {number}
     */
    addChild(child, index= -1) {
        if (child.parent === this) {
            const oldIndex = this.#children.indexOf(child);
            if (oldIndex === index) {
                return;
            }
            index -= oldIndex < index ? 1 : 0;
            this.removeChild(child);
            this.addChild(child, index);
            return;
        }
        if (child.parent) {
            child.parent.removeChild(child);
        }
        child.parent = this;
        if (typeof index === "number" && index >= 0 && index < this.#children.length) {
            view_add_child(this.el, child.el, index);
            this.#children.splice(index, 0, child);
        } else {
            view_add_child(this.el, child.el, -1);
            this.#children.push(child);
        }
    }

    /**
     *
     * @param newNode {View}
     * @param referenceNode {View}
     */
    addChildBefore(newNode, referenceNode) {
        const index = this.#children.indexOf(referenceNode);
        this.addChild(newNode, index);
    }

    /**
     *
     * @param newNode {View}
     * @param referenceNode {View}
     */
    addChildAfter(newNode, referenceNode) {
        const index = this.#children.indexOf(referenceNode);
        if (index >= 0) {
            this.addChild(newNode, index + 1);
        } else {
            this.addChild(newNode);
        }
    }

    /**
     *
     * @param child {View}
     */
    removeChild(child) {
        const index = this.#children.indexOf(child);
        if (index >= 0) {
            child.parent = null;
            view_remove_child(this.el, index);
            this.#children.splice(index, 1);
        } else {
            console.log("remove child failed")
        }
    }
}

export class ContainerElement extends ContainerBasedElement {
    constructor() {
        super(VT_CONTAINER);
    }
}

export class ScrollElement extends ContainerBasedElement {
    constructor() {
        super(VT_SCROLL);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    setScrollX(value) {
        view_set_property(this.el, "scroll_x", value);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    setScrollY(value) {
        view_set_property(this.el, "scroll_y", value);
    }

    scrollBy(value) {
        value.x = value.x || 0;
        value.y = value.y || 0;
        view_set_property(this.el, "scroll_by", value);
    }

}

export class WebSocket {

    client;

    listeners;

    onopen;

    onclose;

    onmessage;

    constructor(url) {
        this.listeners = Object.create(null);
        this._connect(url);
    }

    addEventListener(name, callback) {
        if (!this.listeners[name]) {
            this.listeners[name] = [];
        }
        const listeners = this.listeners[name]
        listeners.push(callback);
    }

    async _connect(url) {
        this.client = await ws_connect(url);
        this._emit("open");
        this._doRead();
    }

    async _doRead() {
        try {
            for (;;) {
                let msg = await ws_read(this.client);
                if (msg === false) {
                    this._emit("close");
                    break;
                }
                let type = typeof msg;
                if (type === "undefined") {
                    continue;
                } else if (type === "string") {
                    this._emit("message", {data: msg});
                }
            }
        } catch (error) {
            console.error(error);
            this._emit("close");
        }
    }

    _emit(name, data) {
        console.log("emit", name, data);
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
            ...data,
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
    console_print("\n");
}

function printObj(value, padding, circleRefList, printedList, level) {
    let type = typeof value;
    if (type === "object" && value != null) {
        const refIdx = circleRefList.indexOf(value);
        if (refIdx >= 0 && printedList.includes(value)) {
            console_print("[Circular *" + refIdx + "]");
        } else {
            const entries = Object.entries(value);
            if (level >= 2) {
                return "{...}"
            }
            if (!entries.length) {
                console_print("{}");
            } else {
                const prefix = refIdx >= 0 ? ("<ref *" + refIdx + ">") : "";
                console_print(prefix + "{\n");
                printedList.push(value);
                entries.forEach(([k, v], index) => {
                    console_print(padding + "  " + k + ":");
                    printObj(v, padding + "  ", circleRefList, printedList, level + 1);
                    if (index < entries.length - 1) {
                        console_print(",\n");
                    }
                });
                console_print("\n" + padding + "}");
            }
        }
    } else if (type === "symbol") {
        console.log("[Symbol]")
    } else if (type === "function") {
        console.log("[Function]")
    } else {
        console_print(value + "");
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

const localStorage = {
    getItem(key) {
        return localstorage_get(key)
    },
    setItem(key, value) {
        localstorage_set(key, value);
    }
}

globalThis.Frame = Frame;
if (globalThis.tray_create) {
    globalThis.SystemTray = SystemTray;
}
globalThis.View = View;
globalThis.ContainerElement = ContainerElement;
globalThis.ScrollElement = ScrollElement;
globalThis.LabelElement = LabelElement;
globalThis.EntryElement = EntryElement;
globalThis.TextEditElement = TextEditElement;
globalThis.ButtonElement = ButtonElement;
globalThis.ImageElement  = ImageElement;
globalThis.Audio = Audio;
globalThis.WebSocket = WebSocket;
globalThis.KEY_MOD_CTRL = 0x1;
globalThis.KEY_MOD_ALT = 0x1 << 1;
globalThis.KEY_MOD_META = 0x1 << 2;
globalThis.KEY_MOD_SHIFT = 0x1 << 3;

globalThis.localStorage = localStorage;