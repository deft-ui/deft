const VT_CONTAINER = 1
const VT_LABEL = 2
const VT_BUTTON = 3
const VT_ENTRY = 4
const VT_GROUP = 5
const VT_PROGRESS_BAR = 6
const VT_SCROLL = 7
const VT_TEXT_EDIT = 8
const VT_IMAGE = 9;

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
export class Navigator {

    /**
     * @var {Clipboard}
     */
    clipboard;
    constructor() {
        this.clipboard = new Clipboard();
    }
}

export class Process {
    /**
     *
     * @param code {number}
     */
    exit(code) {
        process_exit(code);
    }
    get argv() {
        return process_argv();
    }
    get isMobilePlatform() {
        return process_is_mobile_platform();
    }

    /**
     *
     * @param handler {Function}
     */
    setPromiseRejectionTracker(handler) {
        process_set_promise_rejection_tracker(handler);
    }
}


export class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options) {
        return new Promise((resolve, reject) => {
            dialog_show_file_dialog({
                dialogType: options.dialogType,
            }, options.window?.handle, (result, data) => {
                if (result) {
                    resolve(data);
                } else {
                    reject(data);
                }
            })
        })

    }
}

/**
 * @typedef {IEvent<ResizeDetail>} IResizeEvent
 */
export class Window {

    /**
     * @type EventRegistry
     */
    #eventRegistry;

    /**
     * @type EventBinder
     */
    #eventBinder;

    #windowHandle;

    /**
     *
     * @param attrs {WindowAttrs}
     */
    constructor(attrs) {
        this.#windowHandle = Window_create(attrs || {});
        this.#eventBinder = new EventBinder(this.#windowHandle, Window_bind_js_event_listener, Window_unbind_js_event_listener, this);
        Window_set_js_context(this.#windowHandle, this);
    }

    /**
     *
     * @param windowHandle
     * @returns {Window}
     */
    static fromHandle(windowHandle) {
        return Window_get_js_context(windowHandle);
    }

    get handle() {
        return this.#windowHandle
    }

    /**
     *
     * @param element {Element}
     */
    set body(element) {
        Window_set_body(this.#windowHandle, element.handle);
    }

    /**
     *
     * @param title {string}
     */
    set title(title) {
        Window_set_title(this.#windowHandle, title);
    }

    /**
     *
     * @param size {Size}
     */
    resize(size) {
        Window_resize(this.#windowHandle, size);
    }

    /**
     *
     * @param owner {Window}
     */
    setModal(owner) {
        Window_set_modal(this.#windowHandle, owner.#windowHandle)
    }

    close() {
        Window_close(this.#windowHandle);
    }

    /**
     *
     * @param visible {boolean}
     */
    set visible(visible) {
        Window_set_visible(this.#windowHandle, visible);
    }

    requestFullscreen() {
        Window_request_fullscreen(this.#windowHandle);
    }

    exitFullscreen() {
        Window_exit_fullscreen(this.#windowHandle);
    }

    /**
     *
     * @param callback {(event: IResizeEvent) => void}
     */
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

    /**
     * @typedef {("resize", event)} addEventListener
     * @param type
     * @param callback
     */
    addEventListener(type, callback) {
        this.#eventBinder.addEventListener(type, callback);
    }

    removeEventListener(type, callback) {
        this.#eventBinder.removeEventListener(type, callback);
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
    #allEventListeners = Object.create(null);

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
            this.#removeEventListenerApi(this.#target, oldListenerId);
        }
        this.#eventListeners[type] = this.addEventListener(type, callback);
    }
    addEventListener(type, callback) {
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
        if (!this.#allEventListeners[type]) {
            this.#allEventListeners[type] = new Map();
        }
        const id = this.#addEventListenerApi(this.#target, type, eventCallback);
        this.#allEventListeners[type].set(callback, id);
        return id;
    }

    removeEventListener(type, callback) {
        const map = this.#eventListeners[type];
        const id = map.delete(callback);
        if (id) {
            this.#removeEventListenerApi(this.#target, type, id);
        }
    }

}

export class SystemTray {
    /**
     * @type EventRegistry
     */
    #eventRegistry;

    #menuUserCallback;

    tray;
    constructor() {
        this.tray = SystemTray_create("Test");
        this.#eventRegistry = new EventRegistry(this.tray, SystemTray_bind_event, SystemTray_remove_event_listener, this);
    }

    set title(title) {
        SystemTray_set_title(this.tray, title);
    }

    set icon(icon) {
        SystemTray_set_icon(this.tray, icon);
    }

    /**
     *
     * @param menus {TrayMenu[]}
     */
    setMenus(menus) {
        const list = [];
        const menuHandlers = new Map();
        for (const m of menus) {
            const {id, label, checked, enabled} = m;
            const kind = m.kind || "standard";
            if (m.handler) {
                menuHandlers.set(m.id, m.handler);
            }
            list.push({id, label, kind, checked, enabled});
        }
        const menuHandler = (e) => {
            const id = e.detail;
            const handler = menuHandlers.get(id);
            if (handler) {
                handler();
            }
            if (this.#menuUserCallback) {
                this.#menuUserCallback(e);
            }
        }
        SystemTray_set_menus(this.tray, list);
        this.#eventRegistry.bindEvent("menuclick", menuHandler)
    }

    bindActivate(callback) {
        this.#eventRegistry.bindEvent("activate", callback);
    }

    bindMenuClick(callback) {
        this.#menuUserCallback = callback;
    }

}
export class Element {
    /**
     * @type {ContainerBasedElement}
     */
    _parent
    /**
     * @type number
     */
    handle

    /**
     * @type {StyleProps}
     */
    #style

    /**
     * @type EventBinder
     */
    #eventBinder;

    /**
     *
     * @param el {any}
     * @param context {object}
     */
    constructor(el, context) {
        const myContext = this;
        if (typeof el === "number") {
            this.handle = Element_create_by_type(el, myContext);
        } else {
            Element_set_js_context(el, myContext);
            this.handle = el;
        }
        if (!this.handle) {
            throw new Error("Failed to create element:" + el)
        }
        this.#eventBinder = new EventBinder(this.handle, Element_add_js_event_listener, Element_remove_js_event_listener, this, (target) => {
            return Element.fromHandle(target);
        });
    }

    static fromHandle(elementHandle) {
        if (elementHandle) {
            return Element_get_js_context(elementHandle) || null;
        }
        return null;
    }

    createEventBinder(target, addEventListenerApi, removeEventListenerApi) {
        if (!removeEventListenerApi) {
            removeEventListenerApi = (_t, listenerId) => {
                Element_remove_js_event_listener(this.handle, listenerId);
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
     * @returns {number}
     */
    get id() {
        return Element_get_id(this.handle)
    }

    /**
     *
     * @returns {Element | null}
     */
    get parent() {
        const eh = Element_get_parent(this.handle);
        return Element.fromHandle(eh);
    }

    /**
     *
     * @returns {Element}
     */
    get rootElement() {
        let p = this.getParent();
        if (p == null) {
            return this;
        } else {
            return p.getRootElement();
        }
    }

    focus() {
        Element_focus(this.handle);
    }

    get window() {
        const windowHandle = Element_get_window(this.handle);
        return Window.fromHandle(windowHandle);
    }

    /**
     *
     * @param style {StyleProps}
     */
    set style(style) {
        this.#style = style;
        Element_set_style(this.handle, style);
    }

    /**
     *
     * @returns {StyleProps}
     */
    get style() {
        return this.#style
    }

    /**
     *
     * @param style {StyleProps}
     */
    set hoverStyle(style) {
        Element_set_hover_style(this.handle, style);
    }

    /**
     *
     * @param value {number}
     */
    set scrollTop(value) {
        Element_set_scroll_top(this.handle, value);
    }

    /**
     *
     * @param value {number}
     */
    set scrollLeft(value) {
        Element_set_scroll_left(this.handle, value);
    }


    /**
     *
     * @param value {boolean}
     */
    set draggable(value) {
        Element_set_draggable(this.handle, value);
    }

    /**
     *
     * @param value {string}
     */
    set cursor(value) {
        Element_set_cursor(this.handle, value);
    }

    /**
     *
     * @returns {[number, number]}
     */
    get size() {
        return Element_get_size(this.handle);
    }

    /**
     *
     * @returns {[number, number]}
     */
    get contentSize() {
        return Element_get_real_content_size(this.handle);
    }

    /**
     *
     * @returns {ElementRect}
     */
    get boundingClientRect() {
        return Element_get_bounding_client_rect(this.handle);
    }

    /**
     *
     * @returns {number}
     */
    get scrollTop() {
        return Element_get_scroll_top(this.handle);
    }

    /**
     *
     * @returns {number}
     */
    get scrollLeft() {
        return Element_get_scroll_left(this.handle);
    }

    /**
     *
     * @returns {number}
     */
    get scrollHeight() {
        return Element_get_scroll_height(this.handle);
    }

    /**
     *
     * @returns {number}
     */
    get scrollWidth() {
        return Element_scroll_width(this.handle);
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
        this.#eventBinder.bindEvent("click", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindContextMenu(callback) {
        this.#eventBinder.bindEvent("contextmenu", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseDown(callback) {
        this.#eventBinder.bindEvent("mousedown", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseUp(callback) {
        this.#eventBinder.bindEvent("mouseup", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseMove(callback) {
        this.#eventBinder.bindEvent("mousemove", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseEnter(callback) {
        this.#eventBinder.bindEvent("mouseenter", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseLeave(callback) {
        this.#eventBinder.bindEvent("mouseleave", callback);
    }

    bindKeyDown(callback) {
        this.#eventBinder.bindEvent("keydown", callback);
    }

    bindKeyUp(callback) {
        this.#eventBinder.bindEvent("keyup", callback);
    }

    bindSizeChanged(callback) {
        this.#eventBinder.bindEvent("sizechange", callback);
    }

    bindScroll(callback) {
        this.#eventBinder.bindEvent("scroll", callback);
    }

    bindMouseWheel(callback) {
        this.#eventBinder.bindEvent("mousewheel", callback);
    }

    bindDragStart(callback) {
        this.#eventBinder.bindEvent("dragstart", callback);
    }

    bindDragOver(callback) {
        this.#eventBinder.bindEvent("dragover", callback);
    }

    bindDrop(callback) {
        this.#eventBinder.bindEvent("drop", callback);
    }

    bindTouchStart(callback) {
        this.#eventBinder.bindEvent("touchstart", callback);
    }

    bindTouchMove(callback) {
        this.#eventBinder.bindEvent("touchmove", callback);
    }

    bindTouchEnd(callback) {
        this.#eventBinder.bindEvent("touchend", callback);
    }

    bindTouchCancel(callback) {
        this.#eventBinder.bindEvent("touchcancel", callback);
    }

    bindEvent(type, callback) {
        this.#eventBinder.bindEvent(type, callback);
    }

    /**
     *
     * @param value {boolean}
     */
    set autoFocus(value) {
        Element_set_auto_focus(this.handle, value);
    }

    toString() {
        return this.handle + "@" + this.constructor.name
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

export class LabelElement extends Element {
    constructor() {
        super(VT_LABEL);
    }

    /**
     *
     * @param wrap {boolean}
     */
    set textWrap(wrap) {
        Text_set_text_wrap(this.handle, wrap);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        Text_set_text(this.handle, text);
    }

    /**
     *
     * @param align {"left" | "right" | "center"}
     */
    set align(align) {
        Element_set_property(this.handle, "align", align);
    }

    /**
     *
     * @param selection {number[]}
     */
    set selection(selection) {
        Text_set_selection(this.handle, selection);
    }

    /**
     *
     * @param startCaretOffset {number}
     * @param endCaretOffset {number}
     */
    selectByCaretOffset(startCaretOffset, endCaretOffset) {
        this.setSelection([startCaretOffset, endCaretOffset])
    }

    /**
     *
     * @param line {number}
     * @returns {number}
     */
    getLineBeginOffset(line) {
        return Text_get_line_begin_offset(this.handle, line);
    }

    /**
     *
     * @param line {number}
     * @param text {string}
     */
    insertLine(line, text) {
        Text_insert_line(this.handle, line, text);
    }

    /**
     *
     * @param line {number}
     * @param newText {string}
     */
    updateLine(line, newText) {
        Text_update_line(this.handle, line, newText);
    }

    /**
     *
     * @param line {number}
     */
    deleteLine(line) {
        Text_delete_line(this.handle, line);
    }

    /**
     *
     * @param row {number}
     * @param col {number}
     * @return {number}
     */
    getCaretOffsetByCursor(row, col) {
        return Text_get_atom_offset_by_location(this.handle, [row, col]);
    }

}

/**
 * @typedef {{
 *   type: "text",
 *   text: string,
 *   weight ?: string,
 *   textDecorationLine ?: string,
 *   fontFamilies ?: string[],
 *   fontSize ?: number,
 *   color ?: string,
 *   backgroundColor ?: string
 * }} ParagraphUnit
 */
export class ParagraphElement extends Element {
    #paragraph;
    constructor() {
        const p = Paragraph_new_element();
        super(p, {});
        this.#paragraph = p;
    }

    /**
     *
     * @param units {ParagraphUnit[]}
     */
    addLine(units) {
        Paragraph_add_line(this.#paragraph, units);
    }

    /**
     *
     * @param index {number}
     * @param units {ParagraphUnit[]}
     */
    insertLine(index, units) {
        Paragraph_insert_line(this.#paragraph, index, units);
    }


    /**
     *
     * @param index {number}
     */
    deleteLine(index) {
        Paragraph_delete_line(this.#paragraph, index);
    }

    /**
     *
     * @param index {number}
     * @param units {ParagraphUnit[]}
     */
    updateLine(index, units) {
        Paragraph_update_line(this.#paragraph, index, units);
    }

    clear() {
        Paragraph_clear(this.#paragraph);
    }

    /**
     *
     * @param units {ParagraphUnit[]}
     * @return {[number, number]}
     */
    measureLine(units) {
        return Paragraph_measure_line(this.#paragraph, units);
    }

    /**
     *
     * @returns {string | undefined}
     */
    get selectionText() {
        return Paragraph_get_selection_text(this.#paragraph);
    }

}

export class ImageElement extends Element {
    constructor() {
        super(VT_IMAGE);
    }
    set src(src) {
        Image_set_src(this.handle, src);
    }
}


export class EntryElement extends Element {
    constructor() {
        super(VT_ENTRY);
    }

    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    set align(align) {
        Element_set_property(this.handle, "align", align);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        Entry_set_text(this.handle, text);
    }

    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start, end) {
        Entry_set_selection_by_char_offset(this.handle, start, end)
    }

    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset) {
        Entry_set_caret_by_char_offset(this.handle, charOffset);
    }

    /**
     *
     * @param multipleLine {boolean}
     */
    set multipleLine(multipleLine) {
        Entry_set_multiple_line(this.handle, multipleLine)
        // Element_set_property(this.el, "multipleline", String(multipleLine));
    }

    /**
     *
     * @param value {boolean}
     */
    set autoHeight(value) {
        Entry_set_auto_height(this.handle, value);
    }

    /**
     *
     * @returns {string}
     */
    get text() {
        return Entry_get_text(this.handle);
    }

    /**
     *
     * @param rows {number}
     */
    set rows(rows) {
        Entry_set_rows(this.handle, rows);
    }

    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

}

export class TextEditElement extends Element {
    constructor() {
        super(VT_TEXT_EDIT);
    }

    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    set align(align) {
        Element_set_property(this.handle, "align", align);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        Element_set_property(this.handle, "text", text);
    }

    /**
     *
     * @returns {string}
     */
    get text() {
        return Element_get_property(this.handle, "text");
    }

    /**
     *
     * @param selection {[number, number]}
     */
    set selection(selection) {
        Element_set_property(this.handle, "selection", selection);
    }

    /**
     *
     * @param caret {number}
     */
    set caret(caret) {
        Element_set_property(this.handle, "caret", caret);
    }

    /**
     *
     * @param top {number}
     */
    scrollToTop(top) {
        Element_set_property(this.handle, "scroll_to_top", top);
    }

    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

    bindCaretChange(callback) {
        this.bindEvent("caretchange", callback);
    }

}

class ContainerBasedElement extends Element {
    #children = [];
    
    /**
     *
     * @param child {Element}
     * @param index {number}
     */
    addChild(child, index= -1) {
        if (child._parent === this) {
            const oldIndex = this.#children.indexOf(child);
            if (oldIndex === index) {
                return;
            }
            index -= oldIndex < index ? 1 : 0;
            this.removeChild(child);
            this.addChild(child, index);
            return;
        }
        if (child._parent) {
            child._parent.removeChild(child);
        }
        child._parent = this;
        if (typeof index === "number" && index >= 0 && index < this.#children.length) {
            Element_add_child(this.handle, child.handle, index);
            this.#children.splice(index, 0, child);
        } else {
            Element_add_child(this.handle, child.handle, -1);
            this.#children.push(child);
        }
    }

    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildBefore(newNode, referenceNode) {
        const index = this.#children.indexOf(referenceNode);
        this.addChild(newNode, index);
    }

    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
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
     * @param child {Element}
     */
    removeChild(child) {
        const index = this.#children.indexOf(child);
        if (index >= 0) {
            child._parent = null;
            Element_remove_child(this.handle, index);
            this.#children.splice(index, 1);
        } else {
            console.log("remove child failed")
        }
    }
}

export class ButtonElement extends ContainerBasedElement {
    constructor() {
        super(VT_BUTTON);
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
    set scrollX(value) {
        Scroll_set_scroll_x(this.handle, value);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollY(value) {
        Scroll_set_scroll_y(this.handle, value);
    }

    scrollBy(value) {
        value.x = value.x || 0;
        value.y = value.y || 0;
        Element_scroll_by(this.handle, value);
    }

}

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
        //TODO check status
        try {
            await WsConnection_send_str(this.client, data + "");
        } catch (error) {
            this.#emit('error', error);
        }
    }

    close() {
        if (!this.#closed) {
            this.#closed = true;
            this.#emit("close");
            WsConnection_close(this.client);
        }
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
            loop:
            for (;;) {
                let [type, data] = await WsConnection_read(this.client);
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

export class SqliteConn {
    #conn;
    constructor(conn) {
        this.#conn = conn;
    }

    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<number>}
     */
    async execute(sql, params= []) {
        return await SqliteConn_execute(this.#conn, sql, params);
    }

    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<Object[]>}
     */
    async query(sql, params = []) {
        const [columnNames, rows] = await SqliteConn_query(this.#conn, sql, params);
        return rows.map(it => {
            const map = {};
            for (let i = 0; i < columnNames.length; i++) {
                map[columnNames[i]] = it[i];
            }
            return map;
        });
    }

}

export class Sqlite {

    /**
     *
     * @param path {string}
     * @returns {Promise<SqliteConn>}
     */
    static async open(path) {
        const conn = await SqliteConn_open(path);
        return new SqliteConn(conn);
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
    if (value instanceof Error) {
        console.log(`[Error(${value.name})]` + value.message);
        if (value.stack) {
            console.log(value.stack);
        }
    } else if (type === "object" && value != null) {
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
globalThis.process = new Process();
globalThis.process.setPromiseRejectionTracker(error => {
    console.error('uncaught promise error', error);
});
globalThis.fileDialog = new FileDialog();
globalThis.Worker = Worker;
globalThis.WorkerContext = WorkerContext;
globalThis.Window = Window;
if (globalThis.SystemTray_create) {
    globalThis.SystemTray = SystemTray;
}
globalThis.Element = Element;
globalThis.ContainerElement = ContainerElement;
globalThis.ScrollElement = ScrollElement;
globalThis.LabelElement = LabelElement;
globalThis.EntryElement = EntryElement;
globalThis.TextEditElement = TextEditElement;
globalThis.ButtonElement = ButtonElement;
globalThis.ImageElement  = ImageElement;
globalThis.ParagraphElement = ParagraphElement;
globalThis.Audio = Audio;
globalThis.WebSocket = WebSocket;
globalThis.Sqlite = Sqlite;

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
 *     target: Element,
 *     currentTarget: Element,
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