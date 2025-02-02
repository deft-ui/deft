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
class Navigator {

    /**
     * @var {Clipboard}
     */
    clipboard;
    constructor() {
        this.clipboard = new Clipboard();
    }
}

class Process {
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
}


class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options) {
        return new Promise((resolve, reject) => {
            dialog_show_file_dialog({
                dialogType: options.dialogType,
            }, options.frame?.handle, (result, data) => {
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
export class Frame {

    /**
     * @type EventRegistry
     */
    #eventRegistry;

    /**
     * @type EventBinder
     */
    #eventBinder;

    #frameHandle;

    /**
     *
     * @param attrs {FrameAttrs}
     */
    constructor(attrs) {
        this.#frameHandle = Frame_create(attrs || {});
        this.#eventBinder = new EventBinder(this.#frameHandle, Frame_bind_js_event_listener, Frame_unbind_js_event_listener, this);
        Frame_set_js_context(this.#frameHandle, this);
    }

    /**
     *
     * @param frameHandle
     * @returns {Frame}
     */
    static fromHandle(frameHandle) {
        return Frame_get_js_context(frameHandle);
    }

    get handle() {
        return this.#frameHandle
    }

    /**
     *
     * @param view {View}
     */
    setBody(view) {
        Frame_set_body(this.#frameHandle, view.el);
    }

    /**
     *
     * @param title {string}
     */
    setTitle(title) {
        Frame_set_title(this.#frameHandle, title);
    }

    /**
     *
     * @param size {Size}
     */
    resize(size) {
        Frame_resize(this.#frameHandle, size);
    }

    /**
     *
     * @param owner {Frame}
     */
    setModal(owner) {
        Frame_set_modal(this.#frameHandle, owner.#frameHandle)
    }

    close() {
        Frame_close(this.#frameHandle);
    }

    /**
     *
     * @param visible {boolean}
     */
    setVisible(visible) {
        Frame_set_visible(this.#frameHandle, visible);
    }

    requestFullscreen() {
        Frame_request_fullscreen(this.#frameHandle);
    }

    exitFullscreen() {
        Frame_exit_fullscreen(this.#frameHandle);
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

    setTitle(title) {
        SystemTray_set_title(this.tray, title);
    }

    setIcon(icon) {
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
            this.el = Element_create_by_type(el, myContext);
        } else {
            Element_set_js_context(el, myContext);
            this.el = el;
        }
        if (!this.el) {
            throw new Error("Failed to create view:" + el)
        }
        this.#eventBinder = new EventBinder(this.el, Element_add_js_event_listener, Element_remove_js_event_listener, this, (target) => {
            return View.fromHandle(target);
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
     * @returns {number}
     */
    getId() {
        return Element_get_id(this.el)
    }

    /**
     *
     * @returns {View | null}
     */
    getParent() {
        const eh = Element_get_parent(this.el);
        return View.fromHandle(eh);
    }

    /**
     *
     * @returns {View}
     */
    getRootElement() {
        let p = this.getParent();
        if (p == null) {
            return this;
        } else {
            return p.getRootElement();
        }
    }

    focus() {
        Element_focus(this.el);
    }

    getFrame() {
        const frameHandle = Element_get_frame(this.el);
        return Frame.fromHandle(frameHandle);
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
        Element_set_scroll_top(this.el, value);
    }

    /**
     *
     * @param value {number}
     */
    setScrollLeft(value) {
        Element_set_scroll_left(this.el, value);
    }


    /**
     *
     * @param value {boolean}
     */
    setDraggable(value) {
        Element_set_draggable(this.el, value);
    }

    /**
     *
     * @param value {string}
     */
    setCursor(value) {
        Element_set_cursor(this.el, value);
    }

    /**
     *
     * @returns {[number, number]}
     */
    getSize() {
        return Element_get_size(this.el);
    }

    /**
     *
     * @returns {[number, number]}
     */
    getContentSize() {
        return Element_get_real_content_size(this.el);
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
        return Element_get_scroll_top(this.el);
    }

    /**
     *
     * @returns {number}
     */
    getScrollLeft() {
        return Element_get_scroll_left(this.el);
    }

    /**
     *
     * @returns {number}
     */
    getScrollHeight() {
        return Element_get_scroll_height(this.el);
    }

    /**
     *
     * @returns {number}
     */
    getScrollWidth() {
        return Element_scroll_width(this.el);
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
     * @param wrap {boolean}
     */
    setTextWrap(wrap) {
        Text_set_text_wrap(this.el, wrap);
    }

    /**
     *
     * @param text {string}
     */
    setText(text) {
        Text_set_text(this.el, text);
    }

    /**
     *
     * @param align {"left" | "right" | "center"}
     */
    setAlign(align) {
        Element_set_property(this.el, "align", align);
    }

    /**
     *
     * @param selection {number[]}
     */
    setSelection(selection) {
        Text_set_selection(this.el, selection);
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
        return Text_get_line_begin_offset(this.el, line);
    }

    /**
     *
     * @param line {number}
     * @param text {string}
     */
    insertLine(line, text) {
        Text_insert_line(this.el, line, text);
    }

    /**
     *
     * @param line {number}
     * @param newText {string}
     */
    updateLine(line, newText) {
        Text_update_line(this.el, line, newText);
    }

    /**
     *
     * @param line {number}
     */
    deleteLine(line) {
        Text_delete_line(this.el, line);
    }

    /**
     *
     * @param row {number}
     * @param col {number}
     * @return {number}
     */
    getCaretOffsetByCursor(row, col) {
        return Text_get_atom_offset_by_location(this.el, [row, col]);
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
export class ParagraphElement extends View {
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
    getSelectionText() {
        return Paragraph_get_selection_text(this.#paragraph);
    }

}

export class ImageElement extends View {
    constructor() {
        super(VT_IMAGE);
    }
    setSrc(src) {
        Image_set_src(this.el, src);
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
        Entry_set_text(this.el, text);
    }

    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start, end) {
        Entry_set_selection_by_char_offset(this.el, start, end)
    }

    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset) {
        Entry_set_caret_by_char_offset(this.el, charOffset);
    }

    /**
     *
     * @param multipleLine {boolean}
     */
    setMultipleLine(multipleLine) {
        Entry_set_multiple_line(this.el, multipleLine)
        // Element_set_property(this.el, "multipleline", String(multipleLine));
    }

    /**
     *
     * @returns {string}
     */
    getText() {
        return Entry_get_text(this.el);
    }

    /**
     *
     * @param rows {number}
     */
    setRows(rows) {
        Entry_set_rows(this.el, rows);
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
    setScrollX(value) {
        Scroll_set_scroll_x(this.el, value);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    setScrollY(value) {
        Scroll_set_scroll_y(this.el, value);
    }

    scrollBy(value) {
        value.x = value.x || 0;
        value.y = value.y || 0;
        Element_scroll_by(this.el, value);
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
globalThis.fileDialog = new FileDialog();
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