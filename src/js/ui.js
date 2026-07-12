import * as radioGroupApi from "deft:core:radiogroup";
import * as textInputApi from "deft:core:textinput";
import * as containerApi from "deft:core:container";
import * as textEditApi from "deft:core:textedit";
import * as richTextApi from "deft:core:richtext";
import * as elementApi from "deft:core:element";
import * as buttonApi from "deft:core:button";
import * as selectApi from "deft:core:select";
import * as imageApi from "deft:core:image";
import * as radioApi from "deft:core:radio";
import * as labelApi from "deft:core:label";
import * as bodyApi from "deft:core:body";
import * as checkboxApi from "deft:core:checkbox";
import * as windowApi from "deft:core:window"
import * as popupApi from "deft:core:popup";
import * as pageApi from "deft:core:page";
import {EventBinder} from "deft:core";

/**
 * @template T
 * @typedef {{
 *     detail: T,
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
 * @typedef {IEvent<string>} IDroppedFileEvent
 * @typedef {IEvent<string>} IHoveredFileEvent
 * @typedef {IEvent<AppReopenDetail>} IAppReopenEvent
 */


/**
 * @typedef {IEvent<ResizeDetail>} IResizeEvent
 */
export class Window {

    /**
     * @type EventBinder
     */
    #eventBinder;

    #windowHandle;

    #body;

    /**
     *
     * @param attrs {WindowAttrs}
     */
    constructor(attrs = null) {
        attrs = attrs || {};
        attrs.preferredRenderers = [].concat(attrs.preferredRenderers || [])
        this.#windowHandle = windowApi.create(attrs);
        this.#eventBinder = new EventBinder(this.#windowHandle, windowApi.bindJsEventListener, windowApi.unbindJsEventListener, this);
        windowApi.setJsContext(this.#windowHandle, this);
        this.#body = new BodyWidget(() => windowApi.getBodyJs(this.#windowHandle));
    }

    /**
     *
     * @param windowHandle
     * @returns {Window}
     */
    static fromHandle(windowHandle) {
        return windowApi.getJsContext(windowHandle);
    }

    static supportMultipleWindows() {
        return windowApi.supportMultipleWindows();
    }

    get handle() {
        return this.#windowHandle
    }

    /**
     *
     * @returns {BodyWidget}
     */
    get body() {
        return this.#body;
    }


    /**
     *
     * @returns {{width: number, height: number}}
     */
    get innerSize() {
        const [width, height] = windowApi.getInnerSize(this.handle);
        return {width, height}
    }

    /**
     *
     * @param content {Widget}
     * @param x {number}
     * @param y {number}
     * @return {Page}
     */
    createPage(content, x, y) {
        x = x ?? Number.NaN;
        y = y ?? Number.NaN;
        const page = windowApi.createPageJs(this.#windowHandle, content.handle, x, y);
        return new Page(page);
    }

    /**
     *
     * @param content {Widget}
     * @param target {{x: number, y: number, width?: number, height?: number}}
     * @return {Popup}
     */
    popup(content, target) {
        const rect = {
            x: target.x,
            y: target.y,
            width: target.width || 0,
            height: target.height || 0,
        }
        const handle = windowApi.popupJs(this.handle, content.handle, rect);
        return new Popup(handle);
    }

    /**
     *
     * @param menu { import('deft:menu').Menu }
     * @param x {number}
     * @param y {number}
     */
    popupMenu(menu, x, y) {
        windowApi.popupMenu(this.#windowHandle, menu.handle, x, y);
    }

    /**
     *
     * @param message {string | Widget}
     * @param options {AlertOptions}
     */
    showAlert(message, options = {}) {
        options = options || {};
        this.showConfirm(message, {
            ...options,
            hideCancel: true,
        }).finally(() => {
            options?.callback && options.callback();
        });
    }

    /**
     *
     * @param message {string | Widget}
     * @param options {ConfirmOptions}
     * @returns {Promise<boolean>}
     */
    showConfirm(message, options = {}) {
        options = options || {};
        function createBtn(label) {
            const btnLabel = new LabelWidget();
            btnLabel.text = label;
            const btn = new ButtonWidget();
            btn.style = {
                minWidth: '4em',
                flexDirection: 'row',
                justifyContent: 'center',
            }
            btn.addChild(btnLabel);
            return btn;
        }
        return new Promise((resolve) => {
            if (!(message instanceof Widget)) {
                const label = new LabelWidget();
                label.text = message;
                label.style = {
                    padding: '2em',
                }
                message = label;
            }

            const footer = new ContainerWidget();
            footer.style = {
                flexDirection: 'row',
                justifyContent: 'center',
                padding: '10px',
                gap: '2em',
            }
            const btn = createBtn(options?.confirmBtnText ?? "OK");
            footer.addChild(btn);

            let cancelBtn;
            if (!options.hideCancel) {
                cancelBtn = createBtn(options?.cancelBtnText ?? "Cancel");
                footer.addChild(cancelBtn);
            }

            const wrapper = new ContainerWidget();
            wrapper.style = {
                minWidth: 200,
                alignItems: 'center',
            }
            wrapper.addChild(message);
            wrapper.addChild(footer);
            const dialog = this.showDialog(wrapper, options?.title);
            btn.bindClick(() => {
                resolve(true);
                dialog.close();
            });
            cancelBtn?.bindClick(() => {
                resolve(false);
                dialog.close();
            });
        })
    }

    /**
     *
     * @param content {Widget}
     * @param title {string}
     * @returns {{close(): void}}
     */
    showDialog(content, title) {
        if (Window.supportMultipleWindows()) {
            const window = new Window({
                resizable: false,
                preferredRenderers: "SoftBuffer",
                minimizable: false,
                closable: false,
            });
            window.title = title ?? this.title ?? "";
            window.body.addChild(content);
            window.setModal(this);
            window.bindResize((e) => {
                const parentPos = this.outerPosition;
                const parentSize = this.innerSize;
                const x = parentPos.x + (parentSize.width - e.detail.width) / 2.0;
                const y = parentPos.y + (parentSize.height - e.detail.height) / 2.0;
                window.outerPosition = {x, y};
            });
            return {
                close() {
                    window.close();
                }
            }
        } else {
            const wrapper = new DialogWidget();
            if (title) {
                const titleEl = new DialogTitleWidget();
                const titleLabelEl = new LabelWidget();
                titleLabelEl.text = title ?? window.title ?? "";
                titleEl.addChild(titleLabelEl);
                wrapper.addChild(titleEl);
            }
            wrapper.addChild(content);
            const page = this.createPage(wrapper, NaN, NaN);
            return {
                close() {
                    page.close();
                }
            }
        }
    }

    /**
     *
     * @param title {string}
     */
    set title(title) {
        windowApi.setTitle(this.#windowHandle, title);
    }

    /**
     *
     * @returns {string}
     */
    get title() {
        return windowApi.getTitle(this.#windowHandle);
    }

    /**
     *
     * @returns {{width: number, height: number}}
     */
    get monitorSize() {
        const [width, height] = windowApi.getMonitorSize(this.#windowHandle);
        return {
            width,
            height
        }
    }

    /**
     *
     * @param size {Size}
     */
    resize(size) {
        windowApi.resize(this.#windowHandle, size);
    }

    drag() {
        windowApi.drag(this.#windowHandle);
    }

    focus() {
        windowApi.focus(this.#windowHandle);
    }

    /**
     *
     * @returns {boolean}
     */
    get minimized() {
        return windowApi.isMinimized(this.#windowHandle);
    }

    /**
     *
     * @param minimized {boolean}
     */
    set minimized(minimized) {
        windowApi.setMinimized(this.#windowHandle, minimized);
    }

    /**
     *
     * @returns {boolean}
     */
    get maximized() {
        return windowApi.isMaximized(this.#windowHandle);
    }

    /**
     *
     * @param maximized {boolean}
     */
    set maximized(maximized) {
        windowApi.setMaximized(this.#windowHandle, maximized);
    }

    /**
     *
     * @param owner {Window}
     */
    setModal(owner) {
        windowApi.setModal(this.#windowHandle, owner.#windowHandle)
    }

    /**
     *
     * @param value {{x: number, y: number}}
     */
    set outerPosition(value) {
        windowApi.setOuterPosition(this.handle, value.x, value.y);
    }

    /**
     *
     * @returns {{x: number, y: number}}
     */
    get outerPosition() {
        const [x, y] = windowApi.getOuterPosition(this.handle);
        return {x, y}
    }

    close() {
        windowApi.close(this.#windowHandle);
    }

    /**
     *
     * @param visible {boolean}
     */
    set visible(visible) {
        windowApi.setVisible(this.#windowHandle, visible);
    }

    /**
     *
     * @returns {boolean}
     */
    get visible() {
        return windowApi.isVisible(this.#windowHandle);
    }

    requestFullscreen() {
        windowApi.requestFullscreen(this.#windowHandle);
    }

    exitFullscreen() {
        windowApi.exitFullscreen(this.#windowHandle);
    }

    get fullscreen() {
        return windowApi.isFullscreen(this.#windowHandle);
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

export class Popup {
    handle
    constructor(handle) {
        this.handle = handle;
    }
    close() {
        popupApi.close(this.handle);
    }
}

export class Page {
    handle
    constructor(handle) {
        this.handle = handle;
    }
    close() {
        pageApi.close(this.handle);
    }
}

export class Widget {
    /**
     * @type {ContainerBasedWidget}
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
     * @type {StyleProps}
     */
    #hoverStyle

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
        if (typeof el === "function") {
            this.handle = el();
            elementApi.setJsContext(this.handle, myContext);
        } else {
            elementApi.setJsContext(el, myContext);
            this.handle = el;
        }
        if (!this.handle) {
            throw new Error("Failed to create element:" + el)
        }
        this.#eventBinder = new EventBinder(this.handle, elementApi.addJsEventListener, elementApi.removeJsEventListener, this);
    }

    static fromHandle(elementHandle) {
        if (elementHandle) {
            return null;
            // return elementApi.getJsContext(elementHandle) || null;
        }
        return null;
    }

    createEventBinder(target, addEventListenerApi, removeEventListenerApi) {
        if (!removeEventListenerApi) {
            removeEventListenerApi = (_t, listenerId) => {
                elementApi.removeJsEventListener(this.handle, listenerId);
            }
        }
        return new EventBinder(target, addEventListenerApi, removeEventListenerApi, this);
    }

    /**
     * Get the eid of the element
     * @returns {number}
     */
    get eid() {
        return elementApi.getEid(this.handle)
    }

    /**
     *
     * @param clazz {string}
     */
    set class(clazz) {
        elementApi.setClass(this.handle, clazz);
    }

    /**
     *
     * @returns {string}
     */
    get class() {
        return elementApi.getClass(this.handle);
    }

    /**
     *
     * @param clazz {string}
     */
    set className(clazz) {
        elementApi.setClass(this.handle, clazz);
    }

    /**
     *
     * @returns {string}
     */
    get className() {
        return elementApi.getClass(this.handle);
    }

    /**
     * Get the parent of element
     * @returns {Widget | null}
     */
    get parent() {
        const eh = elementApi.getParentWeak(this.handle);
        return Widget.fromHandle(eh);
    }

    /**
     * Make element focusable or not
     * @param focusable {boolean}
     */
    set focusable(focusable) {
        elementApi.setFocusable(this.handle, focusable);
    }

    /**
     * Whether element is focusable
     * @returns {boolean}
     */
    get focusable() {
        return elementApi.isFocusable(this.handle);
    }

    /**
     * Get the root of current element
     * @returns {Widget}
     */
    get rootWidget() {
        let p = this.getParent();
        if (p == null) {
            return this;
        } else {
            return p.getRootElement();
        }
    }

    /**
     * Request focus on the current element
     */
    focus() {
        elementApi.focus(this.handle);
    }

    set tooltip(text) {
        elementApi.setTooltip(this.handle, text);
    }

    get tooltip() {
        return elementApi.getTooltip(this.handle);
    }

    /**
     * Get the window of element
     * @returns {Window}
     */
    get window() {
        const windowHandle = elementApi.getWindow(this.handle);
        return Window.fromHandle(windowHandle);
    }

    /**
     * Set element style
     * @param style {StyleProps | string}
     */
    set style(style) {
        this.#style = style;
        elementApi.setStyle(this.handle, style);
    }

    /**
     * Get element style
     * @returns {StyleProps}
     */
    get style() {
        return elementApi.getStyle(this.handle);
    }

    /**
     * Set element style in hover state
     * @param style {StyleProps | string}
     */
    set hoverStyle(style) {
        this.#hoverStyle = style;
        elementApi.setHoverStyle(this.handle, style);
    }

    /**
     * Get element style in hover state
     * @returns {StyleProps}
     */
    get hoverStyle() {
        return this.#hoverStyle;
    }

    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @param value {number}
     */
    set scrollTop(value) {
        elementApi.setScrollTop(this.handle, value);
    }

    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @returns {number}
     */
    get scrollTop() {
        return elementApi.getScrollTop(this.handle);
    }

    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @param value {number}
     */
    set scrollLeft(value) {
        elementApi.setScrollLeft(this.handle, value);
    }

    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @returns {number}
     */
    get scrollLeft() {
        return elementApi.getScrollLeft(this.handle);
    }

    /**
     * Make element draggable
     * @param value {boolean}
     */
    set draggable(value) {
        elementApi.setDraggable(this.handle, value);
    }

    /**
     * Whether element is draggable or not
     * @returns {*}
     */
    get draggable() {
        return elementApi.getDraggable(this.handle);
    }

    /**
     * Set the cursor in hover state
     * @param value {string}
     */
    set cursor(value) {
        elementApi.setCursor(this.handle, value);
    }

    /**
     * Get the size of element
     * @returns {[number, number]}
     */
    get size() {
        return elementApi.getSize(this.handle);
    }

    /**
     *
     * @returns {[number, number]}
     */
    get contentSize() {
        return elementApi.getRealContentSize(this.handle);
    }

    /**
     *
     * @returns {WidgetRect}
     */
    getBoundingClientRect() {
        return elementApi.getBoundingClientRect(this.handle);
    }

    /**
     * The scrollWidth read-only property is a measurement of the height of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollHeight() {
        return elementApi.getScrollHeight(this.handle);
    }

    /**
     * The scrollWidth read-only property is a measurement of the width of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollWidth() {
        return elementApi.scrollWidth(this.handle);
    }

    setAttribute(key, value) {
        elementApi.setAttribute(this.handle, key, value);
    }

    getAttribute(key) {
        return elementApi.getAttribute(this.handle, key);
    }

    removeAttribute(key) {
        elementApi.removeAttribute(this.handle, key);
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

    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyDown(callback) {
        this.#eventBinder.bindEvent("keydown", callback);
    }

    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
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

    /**
     *
     * @param callback {(e: IDroppedFileEvent) => void}
     */
    bindDroppedFile(callback) {
        this.#eventBinder.bindEvent("droppedfile", callback);
    }

    /**
     *
     * @param callback {(e: IHoveredFileEvent) => void}
     */
    bindHoveredFile(callback) {
        this.#eventBinder.bindEvent("hoveredfile", callback);
    }

    bindEvent(type, callback) {
        this.#eventBinder.bindEvent(type, callback);
    }

    /**
     *
     * @param value {boolean}
     */
    set autoFocus(value) {
        elementApi.setAutoFocus(this.handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get autoFocus() {
        return elementApi.getAutoFocus(this.handle);
    }


    toString() {
        return this.handle + "@" + this.constructor.name
    }

}

export class LabelWidget extends Widget {
    constructor() {
        super(labelApi.create);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        labelApi.setText(this.handle, text);
    }

}

export class CheckboxWidget extends Widget {
    constructor() {
        super(checkboxApi.create);
    }

    /**
     *
     * @param text {string}
     */
    set label(text) {
        checkboxApi.setLabel(this.handle, text);
    }

    /**
     *
     * @returns {string}
     */
    get label() {
        return checkboxApi.getLabel(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set checked(value) {
        checkboxApi.setChecked(this.handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get checked() {
        return checkboxApi.isChecked(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return elementApi.isDisabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        elementApi.setDisabled(this.handle, value);
    }

    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback) {
        this.bindEvent("change", callback);
    }

}

export class RadioWidget extends Widget {
    constructor() {
        super(radioApi.create);
    }

    /**
     *
     * @param text {string}
     */
    set label(text) {
        radioApi.setLabel(this.handle, text);
    }

    /**
     *
     * @returns {string}
     */
    get label() {
        return radioApi.getLabel(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set checked(value) {
        radioApi.setChecked(this.handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get checked() {
        return radioApi.isChecked(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return elementApi.isDisabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        elementApi.setDisabled(this.handle, value);
    }

    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback) {
        this.bindEvent("change", callback);
    }

}

export class SelectWidget extends Widget {
    constructor() {
        super(selectApi.create);
    }

    /**
     *
     * @param value {string}
     */
    set value(value) {
        selectApi.setValue(this.handle, value + "");
    }


    /**
     *
     * @returns {string}
     */
    get value() {
        return selectApi.getValue(this.handle);
    }

    /**
     *
     * @param options {SelectOption[]}
     */
    set options(options) {
        selectApi.setOptions(this.handle, options);
    }

    /**
     *
     * @returns {SelectOption[]}
     */
    get options() {
        return selectApi.getOptions(this.handle);
    }

    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder) {
        selectApi.setPlaceholder(this.handle, placeholder);
    }

    /**
     *
     * @returns {string}
     */
    get placeholder() {
        return selectApi.getPlaceholder(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return elementApi.isDisabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        elementApi.setDisabled(this.handle, value);
    }

    bindChange(callback) {
        this.bindEvent("change", callback);
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
 * }} TextUnit
 */
export class RichTextWidget extends Widget {
    constructor() {
        super(richTextApi.create);
    }

    /**
     *
     * @param units {TextUnit[]}
     */
    addLine(units) {
        richTextApi.addLine(this.handle, units);
    }

    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    insertLine(index, units) {
        richTextApi.insertLine(this.handle, index, units);
    }


    /**
     *
     * @param index {number}
     */
    deleteLine(index) {
        richTextApi.deleteLine(this.handle, index);
    }

    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    updateLine(index, units) {
        richTextApi.updateLine(this.handle, index, units);
    }

    clear() {
        richTextApi.clear(this.handle);
    }

    /**
     *
     * @param units {TextUnit[]}
     * @return {[number, number]}
     */
    measureLine(units) {
        return richTextApi.measureLine(this.handle, units);
    }

    /**
     *
     * @returns {string | undefined}
     */
    get selectionText() {
        return richTextApi.getSelectionText(this.handle);
    }

}

export class ImageWidget extends Widget {
    constructor() {
        super(imageApi.create);
    }
    set src(src) {
        imageApi.setSrc(this.handle, src);
    }
}

export class TextInputWidget extends Widget {

    constructor() {
        super(textInputApi.create);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        textInputApi.setText(this.handle, text);
    }

    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder) {
        textInputApi.setPlaceholder(this.handle, placeholder);
    }

    get placeholder() {
        return textInputApi.getPlaceholder(this.handle);
    }

    /**
     *
     * @param type {"text"|"password"}
     */
    set type(type) {
        textInputApi.setType(this.handle, type);
    }

    /**
     *
     * @returns {"text" | "password"}
     */
    get type() {
        return textInputApi.getType(this.handle);
    }

    /**
     *
     * @returns {string}
     */
    get text() {
        return textInputApi.getText(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return elementApi.isDisabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        elementApi.setDisabled(this.handle, value);
    }


    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback) {
        this.bindEvent("caretchange", callback);
    }

}

export class TextEditWidget extends Widget {
    constructor() {
        super(textEditApi.create);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        textEditApi.setText(this.handle, text);
    }

    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder) {
        textEditApi.setPlaceholder(this.handle, placeholder);
    }

    get placeholder() {
        return textEditApi.getPlaceholder(this.handle);
    }

    /**
     *
     * @returns {number}
     */
    get maxHistory() {
        return textEditApi.getMaxHistory(this.handle);
    }

    /**
     *
     * @param maxHistory {number}
     */
    set maxHistory(maxHistory) {
        textEditApi.setMaxHistory(this.handle, maxHistory);
    }

    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start, end) {
        textEditApi.setSelectionByCharOffset(this.handle, start, end)
    }

    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset) {
        textEditApi.setCaretByCharOffset(this.handle, charOffset);
    }

    /**
     *
     * @returns {string}
     */
    get text() {
        return textEditApi.getText(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return elementApi.isDisabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        elementApi.setDisabled(this.handle, value);
    }


    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback) {
        this.bindEvent("caretchange", callback);
    }

}

export class ContainerBasedWidget extends Widget {
    #children = [];

    /**
     *
     * @param child {Widget}
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
            elementApi.addChildJs(this.handle, child.handle, index);
            this.#children.splice(index, 0, child);
        } else {
            elementApi.addChildJs(this.handle, child.handle, -1);
            this.#children.push(child);
        }
    }

    /**
     *
     * @param newNode {Widget}
     * @param referenceNode {Widget}
     */
    addChildBefore(newNode, referenceNode) {
        const index = this.#children.indexOf(referenceNode);
        this.addChild(newNode, index);
    }

    /**
     *
     * @param newNode {Widget}
     * @param referenceNode {Widget}
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
     * @param child {Widget}
     */
    removeChild(child) {
        const index = this.#children.indexOf(child);
        if (index >= 0) {
            child._parent = null;
            elementApi.removeChild(this.handle, index);
            this.#children.splice(index, 1);
        } else {
            console.log("remove child failed")
        }
    }

    /**
     *
     * @returns {Widget[]}
     */
    get children() {
        return this.#children.slice();
    }

}

export class ButtonWidget extends ContainerBasedWidget {
    constructor() {
        super(buttonApi.create);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return elementApi.isDisabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        elementApi.setDisabled(this.handle, value);
    }

}

export class ContainerWidget extends ContainerBasedWidget {
    constructor() {
        super(containerApi.create);
    }
}

export class DialogWidget extends ContainerBasedWidget {
    constructor() {
        super(() => containerApi.newWithTag("Dialog"));
    }
}

export class DialogTitleWidget extends ContainerBasedWidget {
    constructor() {
        super(() => containerApi.newWithTag("dialog-title"));
    }
}

export class BodyWidget extends ContainerBasedWidget {
    constructor(creator) {
        super(creator || bodyApi.create);
    }
}

export class RadioGroupWidget extends ContainerBasedWidget {
    constructor() {
        super(radioGroupApi.create);
    }
}


globalThis.Window = Window;