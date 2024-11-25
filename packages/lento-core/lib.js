const VT_CONTAINER = 1
const VT_LABEL = 2
const VT_BUTTON = 3
const VT_ENTRY = 4
const VT_GROUP = 5
const VT_PROGRESS_BAR = 6
const VT_SCROLL = 7
const VT_TEXT_EDIT = 8
const VT_IMAGE = 9;

//Note: CONTEXT2ELEMENT must be an weak map to avoid cyclic references.
const CONTEXT2ELEMENT = new WeakMap();

class Clipboard {
    /**
     *
     * @returns {Promise<string>}
     */
    async readText() {
        return clipboard_read_text();
    }

    /**
     *
     * @param text {string}
     * @returns {Promise<void>}
     */
    async writeText(text) {
        clipboard_write_text(text);
    }
}
class Navigator {

    /**
     * @var {Clipboard}
     */
    clipboard;
    constructor() {
        this.clipboard = new Clipboard();
    }
}

export class Frame {

    /**
     * @type EventRegistry
     */
    #eventRegistry;

    /**
     * @type EventBinder
     */
    #eventBinder;

    #frameId;

    /**
     *
     * @param attrs {FrameAttrs}
     */
    constructor(attrs) {
        this.#frameId = Frame_create(attrs || {});
        this.#eventBinder = new EventBinder(this.#frameId, Frame_bind_js_event_listener, Frame_unbind_js_event_listener, this);
    }

    /**
     *
     * @param view {View}
     */
    setBody(view) {
        Frame_set_body(this.#frameId, view.el);
    }

    /**
     *
     * @param title {string}
     */
    setTitle(title) {
        Frame_set_title(this.#frameId, title);
    }

    /**
     *
     * @param size {Size}
     */
    resize(size) {
        Frame_resize(this.#frameId, size);
    }

    /**
     *
     * @param owner {Frame}
     */
    setModal(owner) {
        Frame_set_modal(this.#frameId, owner.#frameId)
    }

    close() {
        Frame_close(this.#frameId);
    }

    /**
     *
     * @param visible {boolean}
     */
    setVisible(visible) {
        Frame_set_visible(this.#frameId, visible);
    }

    bindResize(callback) {
        this.#eventBinder.bindEvent("resize", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindClose(callback) {
        this.bindEvent("close", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback) {
        this.bindEvent("focus", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback) {
        this.bindEvent("blur", callback);
    }

    bindEvent(type, callback) {
        this.#eventBinder.bindEvent(type, callback);
    }

}

/**
 * @template D
 * @template E
 */
export class EventObject {
    _propagationCancelled = false
    _preventDefault = false
    type;
    /**
     * @type {D}
     */
    detail;
    /**
     * @type {E}
     */
    target;

    /**
     * @type {E}
     */
    currentTarget;

    constructor(type, detail, target, currentTarget) {
        this.type = type;
        this.detail = detail;
        this.target = target;
        this.currentTarget = currentTarget;
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
    #contextGetter;
    #self;

    constructor(id, addApi, removeApi, self, contextGetter) {
        this._id = id;
        this._add_api = addApi;
        this._remove_api = removeApi;
        this.#contextGetter = contextGetter;
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

        const getJsContext = (target) => {
            if (target && this.#contextGetter) {
                return this.#contextGetter(target);
            }
            return target;
        }

        const self = this.#self;

        /**
         *
         * @param type {string}
         * @param detail {object}
         * @param target {unknown}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(type, detail, target) {
            const event = new EventObject(type, detail, getJsContext(target), self);
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

export class EventBinder {
    #eventListeners = Object.create(null);
    #target;
    #removeEventListenerApi;
    #addEventListenerApi;
    #contextGetter;
    #self;

    constructor(target, addApi, removeApi, self, contextGetter) {
        this.#target = target;
        this.#addEventListenerApi = addApi;
        this.#removeEventListenerApi = removeApi;
        this.#contextGetter = contextGetter;
        this.#self = self;
    }

    bindEvent(type, callback) {
        type = type.toLowerCase();
        if (typeof callback !== "function") {
            throw new Error("invalid callback");
        }
        let oldListenerId = this.#eventListeners[type];
        if (oldListenerId) {
            this.#removeEventListenerApi(this.#target, type, oldListenerId);
        }

        const getJsContext = (target) => {
            if (target && this.#contextGetter) {
                return this.#contextGetter(target);
            }
            return target;
        }

        const self = this.#self;

        /**
         *
         * @param detail {object}
         * @param target {unknown}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(detail, target) {
            const event = new EventObject(type, detail, getJsContext(target), self);
            try {
                callback && callback(event);
            } catch (error) {
                console.error(`${type} event handling error, detail=`, detail ,error.message || error);
            }
            return event.result();
        }

        this.#eventListeners[type] = this.#addEventListenerApi(this.#target, type, eventCallback);
    }
}

export class SystemTray {
    /**
     * @type EventRegistry
     */
    #eventRegistry;
    tray;
    constructor() {
        this.tray = SystemTray_create("Test");
        this.#eventRegistry = new EventRegistry(this.tray, SystemTray_bind_event, SystemTray_remove_event_listener, this);
    }

    setTitle(title) {
        SystemTray_set_title(this.tray, title);
    }

    setIcon(icon) {
        SystemTray_set_icon(this.tray, icon);
    }

    setMenus(menus) {
        SystemTray_set_menus(this.tray, menus);
    }

    bindActivate(callback) {
        this.#eventRegistry.bindEvent("activate", callback);
    }

    bindMenuClick(callback) {
        this.#eventRegistry.bindEvent("menuclick", callback);
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

    /**
     * @type EventRegistry
     */
    #eventRegistry;

    /**
     *
     * @param el {any}
     * @param context {object}
     */
    constructor(el, context) {
        const myContext = context || {};
        CONTEXT2ELEMENT.set(myContext, this);
        if (typeof el === "number") {
            this.el = Element_create_by_type(el, myContext);
        } else {
            Element_set_js_context(el, myContext);
            this.el = el;
        }
        if (!this.el) {
            throw new Error("Failed to create view:" + el)
        }
        this.#eventRegistry = new EventRegistry(this.el, Element_bind_event, Element_remove_event_listener, this, (target) => {
            const myContext = Element_get_js_context(target);
            if (myContext) {
                return CONTEXT2ELEMENT.get(myContext);
            }
        });
    }

    createEventBinder(target, addEventListenerApi, removeEventListenerApi) {
        if (!removeEventListenerApi) {
            removeEventListenerApi = (_t, listenerId) => {
                Element_remove_js_event_listener(this.el, listenerId);
            }
        }
        return new EventBinder(target, addEventListenerApi, removeEventListenerApi, this, (target) => {
            const myContext = Element_get_js_context(target);
            if (myContext) {
                return CONTEXT2ELEMENT.get(myContext);
            }
        });
    }

    /**
     *
     * @param style {StyleProps}
     */
    setStyle(style) {
        Element_set_style(this.el, style);
    }

    setAnimation(animation) {
        Element_set_animation(this.el, animation);
    }

    /**
     *
     * @param style {StyleProps}
     */
    setHoverStyle(style) {
        Element_set_hover_style(this.el, style);
    }

    /**
     *
     * @param value {number}
     */
    setScrollTop(value) {
        Element_set_property(this.el, "scrollTop", value);
    }

    /**
     *
     * @param value {number}
     */
    setScrollLeft(value) {
        Element_set_property(this.el, "scrollLeft", value);
    }


    /**
     *
     * @param value {boolean}
     */
    setDraggable(value) {
        Element_set_property(this.el, "draggable", value);
    }

    /**
     *
     * @param value {string}
     */
    setCursor(value) {
        Element_set_property(this.el, "cursor", value);
    }

    /**
     *
     * @returns {[number, number]}
     */
    getSize() {
        return Element_get_property(this.el, "size");
    }

    /**
     *
     * @returns {[number, number]}
     */
    getContentSize() {
        return Element_get_property(this.el, "content_size");
    }

    /**
     *
     * @returns {ElementRect}
     */
    getBoundingClientRect() {
        return Element_get_bounding_client_rect(this.el);
    }

    /**
     *
     * @returns {number}
     */
    getScrollTop() {
        return Element_get_property(this.el, "scroll_top");
    }

    /**
     *
     * @returns {number}
     */
    getScrollLeft() {
        return Element_get_property(this.el, "scroll_left");
    }

    /**
     *
     * @returns {number}
     */
    getScrollHeight() {
        return Element_get_property(this.el, "scroll_height");
    }

    /**
     *
     * @returns {number}
     */
    getScrollWidth() {
        return Element_get_property(this.el, "scroll_width");
    }

    /**
     *
     * @param callback {(event: IBoundsChangeEvent) => void}
     */
    bindBoundsChange(callback) {
        this.bindEvent("boundschange", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback) {
        this.bindEvent("focus", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback) {
        this.bindEvent("blur", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindClick(callback) {
        this.bindEvent("click", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseDown(callback) {
        this.bindEvent("mousedown", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseUp(callback) {
        this.bindEvent("mouseup", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseMove(callback) {
        this.bindEvent("mousemove", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseEnter(callback) {
        this.bindEvent("mouseenter", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
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
        this.#eventRegistry.bindEvent(type, callback);
    }

    toString() {
        return this.el + "@" + this.constructor.name
    }

}

export class Audio {
    context;
    #eventRegistry;
    id;
    constructor(config) {
        this.id = Audio_create(config || {})
        this.#eventRegistry = new EventRegistry(this.id, Audio_add_event_listener, Audio_remove_event_listener, this);
    }

    play() {
        Audio_play(this.id);
    }

    pause() {
        Audio_pause(this.id);
    }

    stop() {
        Audio_stop(this.id);
    }

    bindLoad(callback) {
        this.#eventRegistry.bindEvent('load', callback);
    }

    bindTimeUpdate(callback) {
        this.#eventRegistry.bindEvent("timeupdate", callback);
    }

    bindEnd(callback) {
        this.#eventRegistry.bindEvent("end", callback);
    }

    bindPause(callback) {
        this.#eventRegistry.bindEvent("pause", callback);
    }

    bindStop(callback) {
        this.#eventRegistry.bindEvent("stop", callback);
    }

    bindCurrentChange(callback) {
        this.#eventRegistry.bindEvent("currentchange", callback);
    }

    bindEvent(type, callback) {
        this.#eventRegistry.bindEvent(type, callback);
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
        Element_set_property(this.el, "text", text);
    }

    /**
     *
     * @param align {"left" | "right" | "center"}
     */
    setAlign(align) {
        Element_set_property(this.el, "align", align);
    }

    setSelection(selection) {
        Element_set_property(this.el, "selection", selection);
    }

}

export class ImageElement extends View {
    constructor() {
        super(VT_IMAGE);
    }
    setSrc(src) {
        Element_set_property(this.el, "src", src);
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
        Element_set_property(this.el, "title", title);
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
        Element_set_property(this.el, "align", align);
    }

    /**
     *
     * @param text {string}
     */
    setText(text) {
        Element_set_property(this.el, "text", text);
    }

    /**
     *
     * @param multipleLine {boolean}
     */
    setMultipleLine(multipleLine) {
        Element_set_property(this.el, "multipleline", String(multipleLine));
    }

    /**
     *
     * @returns {string}
     */
    getText() {
        return Element_get_property(this.el, "text");
    }

    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

}

export class TextEditElement extends View {
    constructor() {
        super(VT_TEXT_EDIT);
    }

    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    setAlign(align) {
        Element_set_property(this.el, "align", align);
    }

    /**
     *
     * @param text {string}
     */
    setText(text) {
        Element_set_property(this.el, "text", text);
    }

    /**
     *
     * @returns {string}
     */
    getText() {
        return Element_get_property(this.el, "text");
    }

    /**
     *
     * @param selection {[number, number]}
     */
    setSelection(selection) {
        Element_set_property(this.el, "selection", selection);
    }

    /**
     *
     * @param caret {number}
     */
    setCaret(caret) {
        Element_set_property(this.el, "caret", caret);
    }

    /**
     *
     * @param top {number}
     */
    scrollToTop(top) {
        Element_set_property(this.el, "scroll_to_top", top);
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
            Element_add_child(this.el, child.el, index);
            this.#children.splice(index, 0, child);
        } else {
            Element_add_child(this.el, child.el, -1);
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
            Element_remove_child(this.el, index);
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
        Element_set_property(this.el, "scroll_x", value);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    setScrollY(value) {
        Element_set_property(this.el, "scroll_y", value);
    }

    scrollBy(value) {
        value.x = value.x || 0;
        value.y = value.y || 0;
        Element_set_property(this.el, "scroll_by", value);
    }

}

export class WebSocket {

    client;

    listeners;

    onopen;

    onclose;

    onmessage;

    onerror;

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

    send(data) {
        //TODO check status
        WsConnection_send_str(this.client, data + "").catch(error => {
            this.#emit('error', error);
        });
    }

    close() {
        WsConnection_close(this.client);
    }

    async #connect(url) {
        try {
            this.client = await WsConnection_connect(url);
            this.#emit("open");
            this.#doRead();
        } catch (error) {
            this.#emit("error", error);
        }

    }

    async #doRead() {
        try {
            for (;;) {
                let msg = await WsConnection_read(this.client);
                if (msg === false) {
                    this.#emit("close");
                    break;
                }
                let type = typeof msg;
                if (type === "undefined") {
                    continue;
                } else if (type === "string") {
                    this.#emit("message", {data: msg});
                }
            }
        } catch (error) {
            console.error(error);
            this.#emit("close");
        }
    }

    #emit(name, data) {
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

export class Worker {

    #worker

    /**
     * @type EventBinder
     */
    #eventBinder;

    /**
     *
     * @param source {number | string}
     */
    constructor(source) {
        this.#worker = typeof source === "string" ? Worker_create(source) : Worker_bind(source);
        this.#eventBinder = new EventBinder(
            this.#worker,
            Worker_bind_js_event_listener,
            Worker_remove_js_event_listener,
            this
        );
    }

    postMessage(data) {
        Worker_post_message(this.#worker, JSON.stringify(data));
    }

    bindMessage(callback) {
        this.#eventBinder.bindEvent('message', e => {
            e.data = JSON.parse(e.detail.data);
            callback(e);
        });
    }

}

export class WorkerContext {
    #workerContext;
    /**
     * @type {EventBinder}
     */
    #eventBinder;
    constructor() {
        this.#workerContext = WorkerContext_get();
        this.#eventBinder = new EventBinder(
            this.#workerContext,
            WorkerContext_bind_js_event_listener,
            WorkerContext_remove_js_event_listener,
            this
        )
    }
    postMessage(data) {
        WorkerContext_post_message(this.#workerContext, JSON.stringify(data));
    }
    bindMessage(callback) {
        this.#eventBinder.bindEvent('message', e => {
            e.data = JSON.parse(e.detail.data);
            callback(e);
        });
    }

    static create() {
        if (globalThis.WorkerContext_get) {
            return new WorkerContext();
        }
        return null;
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
    Console_print("\n");
}

function printObj(value, padding, circleRefList, printedList, level) {
    let type = typeof value;
    if (type === "object" && value != null) {
        const refIdx = circleRefList.indexOf(value);
        if (refIdx >= 0 && printedList.includes(value)) {
            Console_print("[Circular *" + refIdx + "]");
        } else {
            const entries = Object.entries(value);
            if (level >= 2) {
                return "{...}"
            }
            if (!entries.length) {
                Console_print("{}");
            } else {
                const prefix = refIdx >= 0 ? ("<ref *" + refIdx + ">") : "";
                Console_print(prefix + "{\n");
                printedList.push(value);
                entries.forEach(([k, v], index) => {
                    Console_print(padding + "  " + k + ":");
                    printObj(v, padding + "  ", circleRefList, printedList, level + 1);
                    if (index < entries.length - 1) {
                        Console_print(",\n");
                    }
                });
                Console_print("\n" + padding + "}");
            }
        }
    } else if (type === "symbol") {
        console.log("[Symbol]")
    } else if (type === "function") {
        console.log("[Function]")
    } else {
        Console_print(value + "");
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

/**
 *
 * @type {LocalStorage}
 */
const localStorage = {
    getItem(key) {
        return localstorage_get(key)
    },
    setItem(key, value) {
        localstorage_set(key, value);
    }
}

export const workerContext = WorkerContext.create();
if (workerContext) {
    globalThis.workerContext = workerContext;
}

globalThis.navigator = new Navigator();
globalThis.Worker = Worker;
globalThis.WorkerContext = WorkerContext;
globalThis.Frame = Frame;
if (globalThis.SystemTray_create) {
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
globalThis.setTimeout = globalThis.timer_set_timeout;
globalThis.clearTimeout = globalThis.timer_clear_timeout;
globalThis.setInterval = globalThis.timer_set_interval;
globalThis.clearInterval = globalThis.timer_clear_interval;
globalThis.KEY_MOD_CTRL = 0x1;
globalThis.KEY_MOD_ALT = 0x1 << 1;
globalThis.KEY_MOD_META = 0x1 << 2;
globalThis.KEY_MOD_SHIFT = 0x1 << 3;

globalThis.localStorage = localStorage;

/**
 * @template T
 * @typedef {{
 *     detail: T,
 *     target: View,
 *     currentTarget: View,
 *     stopPropagation(): void,
 *     preventDefault(): void,
 * }} IEvent<T>
 */

/**
 * @typedef {IEvent<BoundsChangeDetail>} IBoundsChangeEvent
 * @typedef {IEvent<void>} IVoidEvent
 * @typedef {IEvent<CaretDetail>} ICaretEvent
 * @typedef {IEvent<MouseDetail>} IMouseEvent
 * @typedef {IEvent<KeyDetail>} IKeyEvent
 * @typedef {IEvent<MouseWheelDetail>} IMouseWheelEvent
 * @typedef {IEvent<TextDetail>} ITextEvent
 * @typedef {IEvent<TouchDetail>} ITouchEvent
 * @typedef {IEvent<ScrollDetail>} IScrollEvent
 */