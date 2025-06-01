const stylesheet = `
.main {
    gap: 10px;
    padding: 10px;
    width: 100%;
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
    const label = new LabelElement();
    label.text = text;
    onClick && label.bindClick(onClick);
    return label;
}

function createTextInput() {
    const input = new TextInputElement();
    input.placeholder = "You can input text here";
    return input;
}

function createPassword() {
    const input = new TextInputElement();
    input.type = "password";
    input.placeholder = "You can input password here"
    return input;
}

function createMultiLineEntry() {
    const textEdit = new TextEditElement();
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
 * @returns {ButtonElement}
 */
function createButton(title, callback) {
    const btn = new ButtonElement();
    const label = new LabelElement();
    label.text = title;
    btn.addChild(label);
    btn.style = {
        width: "6em",
        alignItems: 'center',
    }
    btn.bindClick((e) => {
        console.log("clicked", e);
        callback && callback(e);
    })
    return btn;
}

function createCheckbox(label) {
    const cb = new CheckboxElement();
    // cb.disabled = true;
    cb.label = label;
    return cb;
}

function createRadio(label) {
    const radio = new RadioElement();
    radio.label = label;
    return radio;
}

function createRadioGroup(radioList) {
    const group = new RadioGroupElement();
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
    const img = new ImageElement();
    img.style = {
        width: 32,
        height: 32,
    }
    img.src = "examples/gallery-js/img.svg";
    return img;
}

function createRichText() {
    const richText = new RichTextElement();
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
    const el = new SelectElement();
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
        width: 400,
        height: 400,
    });
    window.title = "Deft Gallery";
    const scroll = new ContainerElement();
    scroll.className = "main";
    window.body.addChild(scroll);

    function createElementRow(label, element, flexDirection = "column") {
        const container = new ContainerElement();
        container.className = "element-row"
        const labelElement = new LabelElement();
        labelElement.text = label;
        labelElement.className = "element-name";
        container.addChild(labelElement);
        if (typeof element === "function") {
            element = element();
        }
        const elementWrapper = new ContainerElement();
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
    });
    const confirmBtn = createButton("Confirm", async () => {
        const result = await window.showConfirm("Are you ok?", {
            confirmBtnText: "Yes",
            cancelBtnText: "No",
        });
        console.log("confirm result:", result);
    })

    const buttonPopup = createButton("Popup", (e) => {
        const label = new LabelElement();
        label.text = "Hello, Deft Gallery!";
        label.style = {
            padding: "4em 2em",
            background: '#ccc',
        }
        window.popup(label, {x: e.detail.windowX, y: e.detail.windowY});
    });
    const checkbox = createCheckbox("Checkbox1");
    const disabledCheckbox = createCheckbox("Disabled");
    const radio1 = createRadio("Rust");
    const radio2 = createRadio("JavaScript");
    const radioGroup = createRadioGroup([radio1, radio2]);
    const select = createSelect();
    disabledCheckbox.bindChange(() => {
        console.log("checked", disabledCheckbox.checked);
        for (const el of [entry, password, multilineEntry, button, confirmBtn, buttonPopup, radio1, radio2, checkbox, select]) {
            el.disabled = disabledCheckbox.checked;
        }
    })

    createElementRow("Label", createLabel("Hello, Deft Gallery!"));
    createElementRow("TextInput", entry);
    createElementRow("Password", password);
    createElementRow("TextEdit", multilineEntry);
    createElementRow("Button", [button, confirmBtn, buttonPopup], "row");
    createElementRow("Radio", radioGroup)
    createElementRow("Select", select);
    createElementRow("Checkbox", [checkbox, disabledCheckbox], "row");
    createElementRow("Image", createImage());
    createElementRow("RichText", createRichText());
}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

