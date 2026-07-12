import {
    ButtonWidget,
    CheckboxWidget, ContainerWidget, ImageWidget,
    LabelWidget,
    RadioWidget,
    RadioGroupWidget, RichTextWidget, SelectWidget,
    TextEditWidget,
    TextInputWidget
} from "deft:ui";

import {Menu, StandardMenuItem} from "deft:menu"

const stylesheet = `
.main {
    gap: 10px;
    padding: 10px;
    width: 100%;
    max-width: 600px;
    margin: 0 auto;
    height: 100%;
    overflow: auto;
}
.element-row {
    flex-direction: row;
    align-items: center;
}
.element-name {
    width: 10em;
}
.element-wrapper {
    flex: 1;
    gap: 10px;
}
`
//TODO support :last selector

function createLabel(text, onClick = null) {
    const label = new LabelWidget();
    label.text = text;
    onClick && label.bindClick(onClick);
    return label;
}

function createTextInput() {
    const input = new TextInputWidget();
    input.placeholder = "You can input text here";
    return input;
}

function createPassword() {
    const input = new TextInputWidget();
    input.type = "password";
    input.placeholder = "You can input password here"
    return input;
}

function createMultiLineEntry() {
    const textEdit = new TextEditWidget();
    textEdit.placeholder = "You can input multiline text here";
    textEdit.style = {
        height: '4em',
    }
    return textEdit;
}

/**
 *
 * @param title
 * @param callback {(e: IMouseEvent) => void}
 * @param tooltip
 * @returns {ButtonWidget}
 */
function createButton(title, callback, tooltip = "") {
    const btn = new ButtonWidget();
    const label = new LabelWidget();
    label.text = title;
    btn.addChild(label);
    btn.tooltip = tooltip;
    btn.style = {
        width: "7em",
        alignItems: 'center',
    }
    btn.bindClick((e) => {
        console.log("clicked", e);
        callback && callback(e);
    })
    return btn;
}

function createCheckbox(label) {
    const cb = new CheckboxWidget();
    // cb.disabled = true;
    cb.label = label;
    return cb;
}

function createRadio(label) {
    const radio = new RadioWidget();
    radio.label = label;
    return radio;
}

function createRadioGroup(radioList) {
    const group = new RadioGroupWidget();
    group.style = {
        flexDirection: 'row',
        gap: '1em',
    }
    for (const r of radioList) {
        group.addChild(r);
    }
    return group;
}

function createImage() {
    const img = new ImageWidget();
    img.style = {
        width: 32,
        height: 32,
    }
    img.src = "res://img.svg";
    return img;
}

function createRichText() {
    const richText = new RichTextWidget();
    richText.style = {
        fontSize: 20,
    }
    richText.addLine([
        {
            type: "text",
            text: "R",
            color: "#F00",
            weight: 'bold',
        },
        {
            type: "text",
            text: "ich",
            weight: 'bold',
        },
        {
            type: "text",
            text: "T",
            color: "#F00",
            style: 'italic',
        },
        {
            type: "text",
            text: "ext",
            style: 'italic',
        }
    ]);
    return richText;
}

function createSelect() {
    const el = new SelectWidget();
    el.options = ["JavaScript", "Rust", "C", "C++", "Java", "Delphi", "C#"].map(it => ({value: it, label: it}));
    el.placeholder = "Select your language...";
    el.bindChange(() => {
        console.log("selected", el.value);
    })
    return el;
}

function main() {
    navigator.stylesheet.append(stylesheet);
    // saveStartTime();
    const window = new Window({
        width: 520,
        height: 400,
    });
    window.title = "Deft Gallery";
    const scroll = new ContainerWidget();
    scroll.className = "main";
    window.body.addChild(scroll);
    if (process.platform === "web") {
        const tip = new LabelWidget();
        tip.text = "No CJK fonts loaded, only English will be displayed";
        tip.style = {
            color: '#F00'
        }
        scroll.addChild(tip);
    }

    function createWidgetRow(label, element, flexDirection = "column") {
        const container = new ContainerWidget();
        container.className = "element-row"
        const labelWidget = new LabelWidget();
        labelWidget.text = label;
        labelWidget.className = "element-name";
        container.addChild(labelWidget);
        if (typeof element === "function") {
            element = element();
        }
        const elementWrapper = new ContainerWidget();
        elementWrapper.style = { flexDirection }
        element = [].concat(element);
        for (const e of element) {
            elementWrapper.addChild(e);
        }
        elementWrapper.className = "element-wrapper";
        container.addChild(elementWrapper);
        scroll.addChild(container);
    }

    const entry = createTextInput();
    const password = createPassword();
    const multilineEntry = createMultiLineEntry();
    const button = createButton("Alert", (e) => {
        window.showAlert("Clicked", {
            title: window.title
        });
    }, "Show alert dialog");
    const confirmBtn = createButton("Confirm", async () => {
        const result = await window.showConfirm("Are you ok?", {
            confirmBtnText: "Yes",
            cancelBtnText: "No",
        });
        console.log("confirm result:", result);
    }, "Show confirm dialog")

    const buttonPopup = createButton("Popup", (e) => {
        const label = new LabelWidget();
        label.text = "Hello, Deft Gallery!";
        label.style = {
            padding: "4em 2em",
        }
        window.popup(label, {x: e.detail.windowX, y: e.detail.windowY});
    });

    const buttonMenu = createButton("Menu", (e) => {
        const menu = new Menu();
        menu.addStandardItem(new StandardMenuItem("Menu1", () => console.log("Menu1 clicked")));
        menu.addSeparator();
        menu.addStandardItem(new StandardMenuItem("Menu2", () => console.log("Menu2 clicked")));
        window.popupMenu(menu, e.detail.windowX, e.detail.windowY);
    });

    const checkbox = createCheckbox("Checkbox1");
    const disabledCheckbox = createCheckbox("Disabled");
    const radio1 = createRadio("Rust");
    const radio2 = createRadio("JavaScript");
    const radioGroup = createRadioGroup([radio1, radio2]);
    const select = createSelect();
    disabledCheckbox.bindChange(() => {
        console.log("checked", disabledCheckbox.checked);
        for (const el of [entry, password, multilineEntry, button, confirmBtn, buttonPopup, buttonMenu, radio1, radio2, checkbox, select]) {
            el.disabled = disabledCheckbox.checked;
        }
    })

    createWidgetRow("Label", createLabel("Hello, Deft Gallery!"));
    createWidgetRow("TextInput", entry);
    createWidgetRow("Password", password);
    createWidgetRow("TextEdit", multilineEntry);
    createWidgetRow("Button", [button, confirmBtn, buttonPopup, buttonMenu], "row");
    createWidgetRow("Radio", radioGroup)
    createWidgetRow("Select", select);
    createWidgetRow("Checkbox", [checkbox, disabledCheckbox], "row");
    createWidgetRow("Image", createImage());
    createWidgetRow("RichText", createRichText());
}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

