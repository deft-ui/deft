const stylesheet= `
.small-entry {
    width: 200px;
}
[color=red] {
    border: #F00 1px;
}
.large-text {
    font-size: 20px;
}
`
function assertEq(expected, actual) {
    if (expected !== actual) {
        console.error(new Error("assert fail"));
        process.exit(1);
    }
}

function runWorker() {
    const worker = new Worker("./worker-index.js");
    worker.bindMessage(data => {
        console.log("receive worker msg", data);
        worker.postMessage("Hello, worker");
    });
}

function createSystemTray() {
    if (!globalThis.SystemTray) {
        return;
    }
    console.log("Setting up tray");
    const tray = new SystemTray();
    tray.icon = "local/deft.ico";
    tray.title="DeftTest";
    tray.bindActivate(() => {
        console.log("tray clicked");
    });
    tray.setMenus([
        {
            id: "Test",
            label: "test",
            handler() {
                console.log("clicked test menu");
            }
        },{
            id: "quit",
            label: "Quit",
            handler() {
                process.exit(0);
            }
        }
    ]);
}

function createEntry() {
    const entry = new EntryElement();
    entry.class = "small-entry";
    entry.placeholder = "Please input some text"
    console.log("entry id:", entry.eid)
    return entry;
}

function createPassword() {
    const entry = new EntryElement();
    entry.class = "small-entry";
    entry.setAttribute("color", "red")
    entry.placeholder = "Input password"
    entry.type = "password";
    console.log("password id:", entry.eid)
    return entry;
}

function createTextEdit() {
    const textEdit = new TextEditElement();
    //textEdit.setAlign("center")
    textEdit.text = "1\n12\n测试\n123\n1234";
    textEdit.autoFocus=true;

    textEdit.style={
        // "height": 100,
        // "width": 100,
        // "background": "#ccc",
        // "minWidth": 600,
        overflow: "hidden",
        height: "",
    };
    console.log("TextEdit id:", textEdit.eid);
    return textEdit;
}

function createCenterElement() {
    const outer = new ContainerElement();
    outer.style={
        position: 'relative',
        height: 200,
        background: '#000',
    };
    const inner = new ContainerElement();
    inner.style={
        position: 'absolute',
        left: '50%',
        top: '50%',
        width: 100,
        height: 100,
        transform: 'translate(-50%, -50%)',
        border: '1 #ccc',
        background: '#ccc',
    };
    outer.addChild(inner);
    console.log("innerId", inner.eid);
    return outer;
}

function createLabel(text, className = "") {
    const label = new LabelElement();
    label.text=text;
    if (className) {
        label.class = className;
    }
    return label;
}

function batchCreateLabels(container) {
    const wrapper = new ContainerElement();
    wrapper.style={
        flexDirection: 'row',
        flexWrap: 'wrap',
    }
    for (let i = 0; i < 2000; i++) {
        const lb = new LabelElement();
        lb.style={
            border: '1 #ccc',
            borderRadius: 10,
            marginTop: 10,
            width: 80,
            height: 20,
        };
        lb.hoverStyle={
            background: '#ccc',
        }
        lb.text = "label" + i;
        lb.bindClick(() => {
            console.log(`clicked label ${i}`)
        })
        wrapper.addChild(lb);
    }
    container.addChild(wrapper);
}

function createAddChildrenButton(container) {
    const button = new ButtonElement();
    button.addChild(createLabel("Add children"));
    button.bindClick(() => {
        batchCreateLabels(container);
        console.log("done");
    });
    return button;
}

function createAnimationButton() {
    let animationButton = new ButtonElement();
    animationButton.style="width: 100;";
    animationButton.addChild(createLabel("Animation"));
    animation_create("rotate", {
        "0": {
            //width: 100,
            transform: 'rotate(0deg)',
            // transform: 'translate(0, 0)',
            // transform: 'scale(1, 1)',
        },
        "1": {
            // width: 200,
            transform: 'rotate(360deg)'
            // transform: 'translate(100%, 0)',
            // transform: 'scale(2, 2)',
        }
    });
    animationButton.hoverStyle={
        animationName: 'rotate',
        animationDuration: 1000,
        animationIterationCount: Infinity,
    }
    console.log("animationButtonId", animationButton.eid);
    return animationButton;
}

function createParagraph() {
    const paragraph = new ParagraphElement();
    paragraph.addLine([
        {
            type: "text",
            text: "Normal",
            fontSize: 20,
            backgroundColor: "#6666",
            // fontFamilies: ["auto-mono"],
        }, {
            type: "text",
            text: "Small red",
            fontSize: 10,
            color: "#F00",
        }
    ]);
    return paragraph;
}

function testWindowHandle(window) {
    const handle = window.handle;
    const windowFromHandle = Window.fromHandle(handle);
    assertEq(window, windowFromHandle);
}

function saveStartTime() {
    const key = "app-start-time";
    const lastStartTime = localStorage.getItem(key);
    console.log("First start:", !lastStartTime);
    if (lastStartTime) {
        console.log("Last start time", lastStartTime);
    }
    localStorage.setItem(key, new Date().getTime() + "");
}

function main() {
    navigator.stylesheet.append(stylesheet);
    // saveStartTime();
    runWorker();
    createSystemTray();
    console.log("begin create window");
    const window = new Window({
        width: 800,
        height: 600,
        // decorations: false,
    });
    testWindowHandle(window);
    window.title="DeftDemo";
    window.bindResize((e) => {
        console.log("window resized", e);
    })
    console.log("window created", window);

    typeface_create("auto-mono", {
        family: "monospace",
        weight: "bold",
    })

    const container = new ScrollElement();
    container.style = {
        flex: 1,
        width: '100%',
        padding: 5,
        gap: 5,
        overflow: 'auto',
    }
    container.bindDroppedFile(e => {
        console.log("dropped file", e);
    })

    container.addChild(createAddChildrenButton(container));
    container.addChild(createAnimationButton());
    container.addChild(createLabel("测试test😃", "large-text"));
    container.addChild(createParagraph());
    container.addChild(createTextEdit());
    const entry = createEntry();
    container.addChild(entry);
    assertEq(container, entry.parent);

    container.addChild(createPassword());
    container.addChild(createCenterElement());
    // batchCreateLabels(container);
    // window.body=(container);
    assertEq(null, window.body.parent);
    window.body.addChild(container);

    assertEq(window, container.window);

}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

