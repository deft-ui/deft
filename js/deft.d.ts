declare type WindowType = "normal" | "menu"
declare type RenderBackend = "SoftBuffer" | "GL" | "SoftGL"
declare interface WindowAttrs {
    width ?: number
    height ?: number
    title ?: string
    resizable ?: boolean,
    decorations ?: boolean,
    overrideRedirect ?: boolean,
    position ?: [number, number],
    visible ?: boolean,
    windowType ?: WindowType,
    minimizable ?: boolean,
    maximizable ?: boolean,
    closable ?: boolean,
    preferredRenderers ?: RenderBackend | RenderBackend[],
}

declare interface ResizeDetail {
    width: number;
    height: number;
}

declare interface ElementRect {
    x: number;
    y: number;
    width: number;
    height: number;
}

declare interface Size {
    width: number;
    height: number;
}

declare interface MouseDetail {
    button: number,
    offsetX: number;
    offsetY: number;
    windowX: number;
    windowY: number;
    screenX: number;
    screenY: number;
}

declare interface CaretDetail {
    position: number,
    originBounds: ElementRect,
    bounds: ElementRect,
}

declare interface KeyDetail {
    modifiers: number,
    ctrlKey: boolean,
    altKey: boolean,
    metaKey: boolean,
    shiftKey: boolean,
    key: string,
    keyStr: string,
    repeat: boolean,
    pressed: boolean,
}

declare interface MouseWheelDetail {
    cols: number;
    rows: number;
}

declare interface TextDetail {
    value: string;
}


declare interface TextChangeDetail {
    value: string;
}

declare interface ScrollDetail {
    scrollTop: number;
    scrollLeft: number;
}

declare interface BoundsChangeDetail {
    originBounds: ElementRect,
}

declare interface TouchInfo {
    identifier: number;
    offsetX: number;
    offsetY: number;
    windowX: number;
    windowY: number;
}

declare interface TouchDetail {
    touches: TouchInfo[],
}

declare type Align =
    'auto'
    | 'flex-start'
    | 'center'
    | 'flex-end'
    | 'stretch'
    | 'baseline'
    | 'space-between'
    | 'space-around'

declare interface SelectOption {
    label: string,
    value: string,
}

declare interface AlertOptions {
    title ?: string;
    confirmBtnText ?: string;
    callback ?: () => void;
}

declare interface ConfirmOptions {
    title ?: string;
    confirmBtnText ?: string;
    cancelBtnText ?: string;
    hideCancel ?: boolean;
}

declare interface StyleProps extends Record<string, number | string>{
    color?: string,
    backgroundColor?: string;
    fontSize?: number;
    lineHeight?: number;

    borderTop?: string;
    borderRight?: string;
    borderBottom?: string;
    borderLeft?: string;

    display?: "none" | "flex",

    width?: number | string,
    height?: number | string,
    maxWidth?: number | string,
    maxHeight?: number | string,
    minWidth?: number | string,
    minHeight?: number | string,

    marginTop?: number | string,
    marginRight?: number | string,
    marginBottom?: number | string,
    marginLeft?: number | string,

    paddingTop?: number | string,
    paddingRight?: number | string,
    paddingBottom?: number | string,
    paddingLeft?: number | string,

    flex?: number,
    flexBasis?: number | string,
    flexGrow?: number,
    flexShrink?: number,
    alignSelf?: Align,
    direction?: 'inherit' | 'ltr' | 'rtl',
    position?: 'static' | 'relative' | 'absolute',
    overflow?: 'visible' | 'hidden' | 'scroll',

    borderTopLeftRadius?: number,
    borderTopRightRadius?: number,
    borderBottomRightRadius?: number,
    borderBottomLeftRadius?: number,

    justifyContent?: 'flex-start' | 'center' | 'flex-end' | 'space-between' | 'space-around' | 'space-evenly',
    flexDirection?: 'column' | 'column-reverse' | 'row' | 'row-reverse',
    alignContent?: Align,
    alignItems?: Align,
    flexWrap?: 'no-wrap' | 'wrap' | 'wrap-reverse',
    columnGap?: number,
    rowGap?: number,
    top?: number | string,
    right?: number | string,
    bottom?: number | string,
    left?: number | string,
    transform?: string,
    animationName?: string,
    animationDuration?: number,
    animationIterationCount?: number,

    // short hands
    background?: string,
    gap?: number,
    border?: string,
    margin?: number | string,
    padding?: number | string,
    borderRadius?: number | string,
}

declare interface LocalStorage {
    getItem(key: string): string | null,

    setItem(key: string, value: string): void,
}
// @ts-ignore
declare const localStorage: LocalStorage;

declare interface TrayMenu {
    kind ?: "standard" | "checkmark" | "separator"
    id ?: string,
    label ?: string,
    checked ?: boolean,
    enabled ?: boolean,
    handler ?: () => void,
}

declare function process_exit(code: number);
declare function path_filename(path: string): string;
declare function path_join(path: string, other: string): string;
declare function animation_create(name: string, keyFrames: Record<string, Record<string, any>>)

declare interface TypefaceParams {
    family: string,
    weight?: string,
}

declare function typeface_create(name: string, params: TypefaceParams): boolean;

declare function env_exe_dir(): String;
declare function env_exe_path(): String;


declare interface UploadOptions {
    file: string,
    field: string,
    data ?: Record<string, string>,
    headers ?: Record<string, string>,
}
declare function http_upload(url: string, options: UploadOptions) : Promise<{status: number, body: string}>;
declare function http_request(url: string) : Promise<any>;
declare interface FetchOptions {
    method ?: 'GET' | 'POST',
    headers ?: Record<string, string>,
    body ?: string,
    proxy ?: string,
}
declare function fetch_create(url: string, options ?: FetchOptions) : Promise<any>;
declare function fetch_response_status(rsp): Promise<number>;
declare function fetch_response_headers(rsp): Promise<{name: string, value: string}[]>;
declare function fetch_response_save(rsp, path: string): Promise<number>;
declare function fetch_response_body_string(rsp): Promise<string>;

declare function AudioRef_create(path: string);
declare function AudioRef_destroy(id): void;
declare function AudioRef_position(id): number;
declare function AudioRef_duration(id): number;

declare function Base64_encode_str(str: string): string;

declare interface ShowFileDialogOptions {
    dialogType ?: "single" | "multiple" | "save" | "dir",
    //TODO fix type
    window ?: any,
}
declare function dialog_show_file_dialog(options ?: ShowFileDialogOptions): Promise<string[]>;

declare function fs_read_dir(path: string): Promise<string[]>;
declare function fs_exists(path: string): Promise<boolean>;
declare function fs_rename(path: string, dest:string): Promise<void>;
declare function fs_delete_file(path: string): Promise<void>;
declare function fs_create_dir(path: string): Promise<void>;
declare function fs_create_dir_all(path: string): Promise<void>;
declare function fs_remove_dir(path: string): Promise<void>;
declare function fs_remove_dir_all(path: string): Promise<void>;


declare function appfs_data_path(path ?: string): string;
declare function appfs_exists(path: string): Promise<boolean>;

declare function appfs_readdir(path: string): Promise<string[]>;

declare function appfs_read(path: string): Promise<string>;

declare function appfs_write_new(path: string, content: string): Promise<void>;

declare function appfs_write(path: string, content: string): Promise<void>;

declare function appfs_delete_file(path: string): Promise<void>;

declare function appfs_create_dir(path: string): Promise<void>;

declare function appfs_create_dir_all(path: string): Promise<void>;

declare function appfs_remove_dir(path: string): Promise<void>;

declare function appfs_remove_dir_all(path: string): Promise<void>;

declare function shell_spawn(executable: string, args ?: string[]): void;

declare function setTimeout(callback: () => void, timeout: number): number;

declare function clearTimeout(timer: number): void;

declare function setInterval(callback: () => void, interval: number): number;

declare function clearInterval(timer: number): void;


declare class Navigator {
    /**
     * @var {Clipboard}
     */
    clipboard: Clipboard;
    /**
     * @var {Stylesheet}
     */
    stylesheet: Stylesheet;
}
declare class Process {
    /**
     *
     * @param code {number}
     */
    exit(code: number): void;
    get argv(): any;
    get isMobilePlatform(): any;
    /**
     *
     * @param handler {Function}
     */
    setPromiseRejectionTracker(handler: Function): void;
}
declare class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options: ShowFileDialogOptions): Promise<string[]>;
}
declare class Page {
    constructor(handle: any);
    handle: any;
    close(): void;
}
declare class Popup {
    constructor(handle: any);
    handle: any;
    close(): void;
}
/**
 * @typedef {IEvent<ResizeDetail>} IResizeEvent
 */
declare class Window {
    /**
     *
     * @param windowHandle
     * @returns {Window}
     */
    static fromHandle(windowHandle: any): Window;
    static supportMultipleWindows(): any;
    /**
     *
     * @param attrs {WindowAttrs}
     */
    constructor(attrs?: WindowAttrs);
    get handle(): any;
    /**
     *
     * @returns {BodyElement}
     */
    get body(): BodyElement;
    /**
     *
     * @returns {{width: number, height: number}}
     */
    get innerSize(): {
        width: number;
        height: number;
    };
    /**
     *
     * @param content {Element}
     * @param x {number}
     * @param y {number}
     * @return {Page}
     */
    createPage(content: Element, x: number, y: number): Page;
    /**
     *
     * @param content {Element}
     * @param target {{x: number, y: number, width?: number, height?: number}}
     * @return {Popup}
     */
    popup(content: Element, target: {
        x: number;
        y: number;
        width?: number;
        height?: number;
    }): Popup;
    /**
     *
     * @param message {string | Element}
     * @param options {AlertOptions}
     */
    showAlert(message: string | Element, options: AlertOptions): void;
    /**
     *
     * @param message {string | Element}
     * @param options {ConfirmOptions}
     * @returns {Promise<boolean>}
     */
    showConfirm(message: string | Element, options: ConfirmOptions): Promise<boolean>;
    /**
     *
     * @param content {Element}
     * @param title {string}
     * @returns {{close(): void}}
     */
    showDialog(content: Element, title: string): {
        close(): void;
    };
    /**
     *
     * @param title {string}
     */
    set title(title: string);
    /**
     *
     * @returns {string}
     */
    get title(): string;
    /**
     *
     * @returns {{width: number, height: number}}
     */
    get monitorSize(): {
        width: number;
        height: number;
    };
    /**
     *
     * @param size {Size}
     */
    resize(size: Size): void;
    drag(): void;
    /**
     *
     * @param minimized {boolean}
     */
    set minimized(minimized: boolean);
    /**
     *
     * @returns {boolean}
     */
    get minimized(): boolean;
    /**
     *
     * @param maximized {boolean}
     */
    set maximized(maximized: boolean);
    /**
     *
     * @returns {boolean}
     */
    get maximized(): boolean;
    /**
     *
     * @param owner {Window}
     */
    setModal(owner: Window): void;
    /**
     *
     * @param value {{x: number, y: number}}
     */
    set outerPosition(value: {
        x: number;
        y: number;
    });
    /**
     *
     * @returns {{x: number, y: number}}
     */
    get outerPosition(): {
        x: number;
        y: number;
    };
    close(): void;
    /**
     *
     * @param visible {boolean}
     */
    set visible(visible: boolean);
    /**
     *
     * @returns {boolean}
     */
    get visible(): boolean;
    requestFullscreen(): void;
    exitFullscreen(): void;
    get fullscreen(): any;
    /**
     *
     * @param callback {(event: IResizeEvent) => void}
     */
    bindResize(callback: (event: IResizeEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindClose(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback: (event: IVoidEvent) => void): void;
    bindEvent(type: any, callback: any): void;
    /**
     * @typedef {("resize", event)} addEventListener
     * @param type
     * @param callback
     */
    addEventListener(type: any, callback: any): void;
    removeEventListener(type: any, callback: any): void;
    
}
/**
 * @template D
 * @template E
 */
declare class EventObject<D, E> {
    constructor(type: any, detail: any, target: any, currentTarget: any);
    _propagationCancelled: boolean;
    _preventDefault: boolean;
    type: any;
    /**
     * @type {D}
     */
    detail: D;
    /**
     * @type {E}
     */
    target: E;
    /**
     * @type {E}
     */
    currentTarget: E;
    stopPropagation(): void;
    preventDefault(): void;
    result(): {
        propagationCancelled: boolean;
        preventDefault: boolean;
    };
}
declare class EventRegistry {
    constructor(id: any, addApi: any, removeApi: any, self: any, contextGetter: any);
    eventListeners: any;
    _id: any;
    _remove_api: any;
    _add_api: any;
    bindEvent(type: any, callback: any): void;
    
}
declare class EventBinder {
    constructor(target: any, addApi: any, removeApi: any, self: any, contextGetter: any);
    bindEvent(type: any, callback: any): void;
    addEventListener(type: any, callback: any): any;
    removeEventListener(type: any, callback: any): void;
    
}
declare class SystemTray {
    tray: any;
    set title(title: any);
    set icon(icon: any);
    /**
     *
     * @param menus {TrayMenu[]}
     */
    setMenus(menus: TrayMenu[]): void;
    bindActivate(callback: any): void;
    bindMenuClick(callback: any): void;
    
}
declare class Element {
    static fromHandle(elementHandle: any): any;
    /**
     *
     * @param el {any}
     * @param context {object}
     */
    constructor(el: any, context: object);
    /**
     * @type {ContainerBasedElement}
     */
    _parent: ContainerBasedElement;
    /**
     * @type number
     */
    handle: number;
    createEventBinder(target: any, addEventListenerApi: any, removeEventListenerApi: any): EventBinder;
    /**
     * Get the eid of the element
     * @returns {number}
     */
    get eid(): number;
    /**
     *
     * @param clazz {string}
     */
    set class(clazz: string);
    /**
     *
     * @returns {string}
     */
    get class(): string;
    /**
     *
     * @param clazz {string}
     */
    set className(clazz: string);
    /**
     *
     * @returns {string}
     */
    get className(): string;
    /**
     * Get the parent of element
     * @returns {Element | null}
     */
    get parent(): Element | null;
    /**
     * Make element focusable or not
     * @param focusable {boolean}
     */
    set focusable(focusable: boolean);
    /**
     * Whether element is focusable
     * @returns {boolean}
     */
    get focusable(): boolean;
    /**
     * Get the root of current element
     * @returns {Element}
     */
    get rootElement(): Element;
    /**
     * Request focus on the current element
     */
    focus(): void;
    /**
     * Get the window of element
     * @returns {Window}
     */
    get window(): Window;
    /**
     * Set element style
     * @param style {StyleProps | string}
     */
    set style(style: StyleProps | string);
    /**
     * Get element style
     * @returns {StyleProps}
     */
    get style(): StyleProps;
    /**
     * Set element style in hover state
     * @param style {StyleProps | string}
     */
    set hoverStyle(style: StyleProps | string);
    /**
     * Get element style in hover state
     * @returns {StyleProps}
     */
    get hoverStyle(): StyleProps;
    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @param value {number}
     */
    set scrollTop(value: number);
    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @returns {number}
     */
    get scrollTop(): number;
    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @param value {number}
     */
    set scrollLeft(value: number);
    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @returns {number}
     */
    get scrollLeft(): number;
    /**
     * Make element draggable
     * @param value {boolean}
     */
    set draggable(value: boolean);
    /**
     * Whether element is draggable or not
     * @returns {*}
     */
    get draggable(): any;
    /**
     * Set the cursor in hover state
     * @param value {string}
     */
    set cursor(value: string);
    /**
     * Get the size of element
     * @returns {[number, number]}
     */
    get size(): [number, number];
    /**
     *
     * @returns {[number, number]}
     */
    get contentSize(): [number, number];
    /**
     *
     * @returns {ElementRect}
     */
    getBoundingClientRect(): ElementRect;
    /**
     * The scrollWidth read-only property is a measurement of the height of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollHeight(): number;
    /**
     * The scrollWidth read-only property is a measurement of the width of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollWidth(): number;
    setAttribute(key: any, value: any): void;
    getAttribute(key: any): any;
    removeAttribute(key: any): void;
    /**
     *
     * @param callback {(event: IBoundsChangeEvent) => void}
     */
    bindBoundsChange(callback: (event: IBoundsChangeEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindClick(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindContextMenu(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseDown(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseUp(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseMove(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseEnter(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseLeave(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyDown(callback: (e: IKeyEvent) => void): void;
    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyUp(callback: (e: IKeyEvent) => void): void;
    bindSizeChanged(callback: any): void;
    bindScroll(callback: any): void;
    bindMouseWheel(callback: any): void;
    bindDragStart(callback: any): void;
    bindDragOver(callback: any): void;
    bindDrop(callback: any): void;
    bindTouchStart(callback: any): void;
    bindTouchMove(callback: any): void;
    bindTouchEnd(callback: any): void;
    bindTouchCancel(callback: any): void;
    /**
     *
     * @param callback {(e: IDroppedFileEvent) => void}
     */
    bindDroppedFile(callback: (e: IDroppedFileEvent) => void): void;
    /**
     *
     * @param callback {(e: IHoveredFileEvent) => void}
     */
    bindHoveredFile(callback: (e: IHoveredFileEvent) => void): void;
    bindEvent(type: any, callback: any): void;
    /**
     *
     * @param value {boolean}
     */
    set autoFocus(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get autoFocus(): boolean;
    toString(): string;
    
}
declare class Audio {
    constructor(config: any);
    context: any;
    id: any;
    play(): void;
    pause(): void;
    stop(): void;
    bindLoad(callback: any): void;
    bindTimeUpdate(callback: any): void;
    bindEnd(callback: any): void;
    bindPause(callback: any): void;
    bindStop(callback: any): void;
    bindCurrentChange(callback: any): void;
    bindEvent(type: any, callback: any): void;
    
}
declare class LabelElement extends Element {
    constructor();
    /**
     *
     * @param text {string}
     */
    set text(text: string);
}
declare class CheckboxElement extends Element {
    constructor();
    /**
     *
     * @param text {string}
     */
    set label(text: string);
    /**
     *
     * @returns {string}
     */
    get label(): string;
    /**
     *
     * @param value {boolean}
     */
    set checked(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get checked(): boolean;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback: (e: IVoidEvent) => void): void;
}
declare class RadioElement extends Element {
    constructor();
    /**
     *
     * @param text {string}
     */
    set label(text: string);
    /**
     *
     * @returns {string}
     */
    get label(): string;
    /**
     *
     * @param value {boolean}
     */
    set checked(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get checked(): boolean;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback: (e: IVoidEvent) => void): void;
}
declare class SelectElement extends Element {
    constructor();
    /**
     *
     * @param value {string}
     */
    set value(value: string);
    /**
     *
     * @returns {string}
     */
    get value(): string;
    /**
     *
     * @param options {SelectOption[]}
     */
    set options(options: SelectOption[]);
    /**
     *
     * @returns {SelectOption[]}
     */
    get options(): SelectOption[];
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    /**
     *
     * @returns {string}
     */
    get placeholder(): string;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    bindChange(callback: any): void;
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
 *
 * @deprecated
 */
declare class ParagraphElement extends Element {
    constructor();
    /**
     *
     * @param units {ParagraphUnit[]}
     */
    addLine(units: ParagraphUnit[]): void;
    /**
     *
     * @param index {number}
     * @param units {ParagraphUnit[]}
     */
    insertLine(index: number, units: ParagraphUnit[]): void;
    /**
     *
     * @param index {number}
     */
    deleteLine(index: number): void;
    /**
     *
     * @param index {number}
     * @param units {ParagraphUnit[]}
     */
    updateLine(index: number, units: ParagraphUnit[]): void;
    clear(): void;
    /**
     *
     * @param units {ParagraphUnit[]}
     * @return {[number, number]}
     */
    measureLine(units: ParagraphUnit[]): [number, number];
    /**
     *
     * @returns {string | undefined}
     */
    get selectionText(): string | undefined;
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
declare class RichTextElement extends Element {
    constructor();
    /**
     *
     * @param units {TextUnit[]}
     */
    addLine(units: TextUnit[]): void;
    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    insertLine(index: number, units: TextUnit[]): void;
    /**
     *
     * @param index {number}
     */
    deleteLine(index: number): void;
    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    updateLine(index: number, units: TextUnit[]): void;
    clear(): void;
    /**
     *
     * @param units {TextUnit[]}
     * @return {[number, number]}
     */
    measureLine(units: TextUnit[]): [number, number];
    /**
     *
     * @returns {string | undefined}
     */
    get selectionText(): string | undefined;
}
declare class ImageElement extends Element {
    constructor();
    set src(src: any);
}
declare class EntryElement extends Element {
    constructor();
    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    set align(align: "left" | "right" | "center");
    /**
     *
     * @param text {string}
     */
    set text(text: string);
    /**
     *
     * @returns {string}
     */
    get text(): string;
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    get placeholder(): string;
    /**
     *
     * @param style {StyleProps}
     */
    set placeholderStyle(style: StyleProps);
    /**
     *
     * @returns {StyleProps}
     */
    get placeholderStyle(): StyleProps;
    /**
     *
     * @param type {"text"|"password"}
     */
    set type(type: "text" | "password");
    /**
     *
     * @returns {"text" | "password"}
     */
    get type(): "text" | "password";
    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start: number, end: number): void;
    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset: number): void;
    /**
     *
     * @param multipleLine {boolean}
     */
    set multipleLine(multipleLine: boolean);
    /**
     *
     * @param value {boolean}
     */
    set autoHeight(value: boolean);
    /**
     *
     * @param rows {number}
     */
    set rows(rows: number);
    set disabled(value: any);
    get disabled(): any;
    bindTextChange(callback: any): void;
    bindCaretChange(callback: any): void;
    
}
declare class TextInputElement extends Element {
    constructor();
    /**
     *
     * @param text {string}
     */
    set text(text: string);
    /**
     *
     * @returns {string}
     */
    get text(): string;
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    get placeholder(): string;
    /**
     *
     * @param type {"text"|"password"}
     */
    set type(type: "text" | "password");
    /**
     *
     * @returns {"text" | "password"}
     */
    get type(): "text" | "password";
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback: (e: ITextEvent) => void): void;
    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback: (e: ICaretEvent) => void): void;
}
declare class TextEditElement extends Element {
    constructor();
    /**
     *
     * @param text {string}
     */
    set text(text: string);
    /**
     *
     * @returns {string}
     */
    get text(): string;
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    get placeholder(): string;
    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start: number, end: number): void;
    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset: number): void;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback: (e: ITextEvent) => void): void;
    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback: (e: ICaretEvent) => void): void;
}
declare class ButtonElement extends ContainerBasedElement {
    constructor();
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
}
declare class ContainerElement extends ContainerBasedElement {
    constructor();
}
declare class DialogElement extends ContainerBasedElement {
    constructor();
}
declare class DialogTitleElement extends ContainerBasedElement {
    constructor();
}
declare class BodyElement extends ContainerBasedElement {
    constructor();
}
declare class ScrollElement extends ContainerBasedElement {
    constructor();
    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollX(value: "auto" | "always" | "never");
    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollY(value: "auto" | "always" | "never");
    scrollBy(value: any): void;
}
declare class RadioGroupElement extends ContainerBasedElement {
    constructor();
}
declare class WebSocket {
    constructor(url: any);
    client: any;
    listeners: any;
    onopen: any;
    onclose: any;
    onmessage: any;
    onping: any;
    onpong: any;
    onerror: any;
    addEventListener(name: any, callback: any): void;
    send(data: any): Promise<void>;
    close(): void;
    
}
declare class Worker {
    /**
     *
     * @param source {number | string}
     */
    constructor(source: number | string);
    postMessage(data: any): void;
    bindMessage(callback: any): void;
    
}
declare class WorkerContext {
    static create(): WorkerContext;
    postMessage(data: any): void;
    bindMessage(callback: any): void;
    
}
declare class SqliteConn {
    constructor(conn: any);
    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<number>}
     */
    execute(sql: string, params?: any[]): Promise<number>;
    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<Object[]>}
     */
    query(sql: string, params?: any[]): Promise<any[]>;
    
}
declare class Sqlite {
    /**
     *
     * @param path {string}
     * @returns {Promise<SqliteConn>}
     */
    static open(path: string): Promise<SqliteConn>;
}
declare const workerContext: WorkerContext;
declare class FetchResponse {
    constructor(resp: any, status: any);
    _resp: any;
    status: any;
    ok: boolean;
    json(): Promise<any>;
}
declare type IResizeEvent = IEvent<ResizeDetail>;
declare type ParagraphUnit = {
    type: "text";
    text: string;
    weight?: string;
    textDecorationLine?: string;
    fontFamilies?: string[];
    fontSize?: number;
    color?: string;
    backgroundColor?: string;
};
declare type TextUnit = {
    type: "text";
    text: string;
    weight?: string;
    textDecorationLine?: string;
    fontFamilies?: string[];
    fontSize?: number;
    color?: string;
    backgroundColor?: string;
};
/**
 * <T>
 */
declare type IEvent<T> = {
    detail: T;
    target: Element;
    currentTarget: Element;
    stopPropagation(): void;
    preventDefault(): void;
};
declare type IBoundsChangeEvent = IEvent<BoundsChangeDetail>;
declare type IVoidEvent = IEvent<void>;
declare type ICaretEvent = IEvent<CaretDetail>;
declare type IMouseEvent = IEvent<MouseDetail>;
declare type IKeyEvent = IEvent<KeyDetail>;
declare type IMouseWheelEvent = IEvent<MouseWheelDetail>;
declare type ITextEvent = IEvent<TextDetail>;
declare type ITouchEvent = IEvent<TouchDetail>;
declare type IScrollEvent = IEvent<ScrollDetail>;
declare type IDroppedFileEvent = IEvent<string>;
declare type IHoveredFileEvent = IEvent<string>;
declare class Clipboard {
    /**
     *
     * @returns {Promise<string>}
     */
    readText(): Promise<string>;
    /**
     *
     * @param text {string}
     * @returns {Promise<void>}
     */
    writeText(text: string): Promise<void>;
}
declare class Stylesheet {
    /**
     *
     * @param code {string}
     * @returns {StylesheetItem}
     */
    append(code: string): StylesheetItem;
    /**
     *
     * @param stylesheet {StylesheetItem}
     */
    remove(stylesheet: StylesheetItem): void;
}
declare class ContainerBasedElement extends Element {
    /**
     *
     * @param child {Element}
     * @param index {number}
     */
    addChild(child: Element, index?: number): void;
    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildBefore(newNode: Element, referenceNode: Element): void;
    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildAfter(newNode: Element, referenceNode: Element): void;
    /**
     *
     * @param child {Element}
     */
    removeChild(child: Element): void;
    /**
     *
     * @returns {Element[]}
     */
    get children(): Element[];
    
}
declare class StylesheetItem {
    constructor(id: any);
    id: any;
    update(code: any): void;
}

