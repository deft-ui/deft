const stylesheet = `
.main {
    gap: 10px;
    padding: 10px;
    width: 100%;
    height: 100%;
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
    flex-direction: row;
    gap: 10px;
} 
radiogroup {
    flex-direction: row;
    gap: 10px;
}
`

function createLabel() {
    const label = new LabelElement();
    label.text = "Hello, Deft Gallery!";
    return label;
}

function createEntry() {
    const entry = new EntryElement();
    entry.placeholder = "You can input text here";
    return entry;
}

function createPassword() {
    const entry = new EntryElement();
    entry.type = "password";
    entry.placeholder = "You can input password here"
    return entry;
}

function createMultiLineEntry() {
    const entry = new EntryElement();
    entry.placeholder = "You can input multiline text here";
    entry.multipleLine = true;
    entry.style = {
        height: '4em',
    }
    return entry;
}

function createButton() {
    const btn = new ButtonElement();
    const label = new LabelElement();
    label.text = "Click Me";
    btn.addChild(label);
    btn.style = {
        width: "8em",
        alignItems: 'center',
    }
    btn.bindClick((e) => {
        console.log("clicked", e)
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

function createParagraph() {
    const paragraph = new ParagraphElement();
    paragraph.style = {
        fontSize: 20,
    }
    paragraph.addLine([
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
    return paragraph;
}

function main() {
    navigator.stylesheet.append(stylesheet);
    // saveStartTime();
    const window = new Window({
        width: 400,
        height: 400,
    });
    window.title = "Deft Gallery";
    const scroll = new ScrollElement();
    scroll.className = "main";
    window.body.addChild(scroll);

    function createElementRow(label, element) {
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
        element = [].concat(element);
        for (const e of element) {
            elementWrapper.addChild(e);
        }
        elementWrapper.className = "element-wrapper";
        container.addChild(elementWrapper);
        scroll.addChild(container);
    }

    const entry = createEntry();
    const password = createPassword();
    const multilineEntry = createMultiLineEntry();
    const button = createButton();
    const checkbox = createCheckbox("Checkbox1");
    const disabledCheckbox = createCheckbox("Disabled");
    const radio1 = createRadio("Rust");
    const radio2 = createRadio("JavaScript");
    const radioGroup = createRadioGroup([radio1, radio2]);
    disabledCheckbox.bindChange(() => {
        console.log("checked", disabledCheckbox.checked);
        for (const el of [entry, password, multilineEntry, button, radio1, radio2, checkbox]) {
            el.disabled = disabledCheckbox.checked;
        }
    })


    createElementRow("Label", createLabel);
    createElementRow("Entry", entry);
    createElementRow("Password", password);
    createElementRow("Multiline Entry", multilineEntry);
    createElementRow("Button", button);
    createElementRow("Radio", radioGroup)
    createElementRow("Checkbox", [checkbox, disabledCheckbox]);
    createElementRow("Image", createImage());
    createElementRow("Paragraph", createParagraph());
}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

