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
    const tray = new SystemTray();
    tray.title="DeftTest";
    tray.setMenus([{
        id: "test",
        label: "test",
        handler() {
            console.log("clicked test menu");
        }
    }]);
}

function createEntry() {
    const entry = new EntryElement();
    entry.text = "test测试";
    entry.style = {
        width: 100,
        padding: 5,
        border: "1 #ccc"
    }
    console.log("entry id:", entry.id)
    return entry;
}

function createTextEdit() {
    const textEdit = new EntryElement();

    //textEdit.setAlign("center")
    textEdit.text = "1\n12\n测试\n123\n1234";
    textEdit.multipleLine=true;
    textEdit.autoHeight=true;
    textEdit.autoFocus=true;

    textEdit.style={
        padding: 10,
        // "height": 100,
        // "width": 100,
        // "background": "#ccc",
        "border": "1 #ccc"
        // "minWidth": 600,
    };
    console.log("TextEdit id:", textEdit.id);
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
    console.log("innerId", inner.id);
    return outer;
}

function createLabel(text) {
    const label = new LabelElement();
    label.text=text;
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
    animationButton.style={
        width: 100,
    };
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
    console.log("animationButtonId", animationButton.id);
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
            fontFamilies: ["auto-mono"],
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

function main() {
    runWorker();
    createSystemTray();
    console.log("begin create window");
    const window = new Window();
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
    container.style={
        background: "#2a2a2a",
        color: "#FFF",
        padding: 5,
        gap: 5,
    }

    container.addChild(createAddChildrenButton(container));
    container.addChild(createAnimationButton());
    container.addChild(createLabel("测试test"));
    container.addChild(createParagraph());
    container.addChild(createTextEdit());
    const entry = createEntry();
    container.addChild(entry);
    assertEq(container, entry.parent);
    container.addChild(createCenterElement());
    // batchCreateLabels(container);
    window.body=(container);
    assertEq(window, container.window);
    assertEq(null, container.parent);
}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

