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
    tray.setTitle("LentoTest");
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
    entry.setText("test测试");
    entry.setStyle({
        width: 100,
        padding: 5,
        border: "1 #ccc"
    })
    console.log("entry id:", entry.getId())
    return entry;
}

function createTextEdit() {
    const textEdit = new EntryElement();

    //textEdit.setAlign("center")
    textEdit.setText("1\n12\n测试\n123\n1234");
    textEdit.setMultipleLine(true);

    textEdit.setStyle({
        padding: 10,
        // "height": 100,
        // "width": 100,
        // "background": "#ccc",
        "border": "1 #ccc"
        // "minWidth": 600,
    });
    console.log("TextEdit id:", textEdit.getId());
    return textEdit;
}

function createCenterElement() {
    const outer = new ContainerElement();
    outer.setStyle({
        position: 'relative',
        height: 200,
        background: '#000',
    });
    const inner = new ContainerElement();
    inner.setStyle({
        position: 'absolute',
        left: '50%',
        top: '50%',
        width: 100,
        height: 100,
        transform: 'translate(-50%, -50%)',
        border: '1 #ccc',
        background: '#ccc',
    });
    outer.addChild(inner);
    console.log("innerId", inner.getId());
    return outer;
}

function createLabel(text) {
    const label = new LabelElement();
    label.setText(text);
    return label;
}

function batchCreateLabels(container) {
    const wrapper = new ContainerElement();
    wrapper.setStyle({
        flexDirection: 'row',
        flexWrap: 'wrap',
    })
    for (let i = 0; i < 2000; i++) {
        const lb = new LabelElement();
        lb.setStyle({
            border: '1 #ccc',
            borderRadius: 10,
            marginTop: 10,
            width: 80,
            height: 20,
        });
        lb.setHoverStyle({
            background: '#ccc',
        })
        lb.setText("label" + i);
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
    animationButton.setStyle({
        width: 100,
    });
    animationButton.addChild(createLabel("Animation"));
    animation_create("rotate", {
        "0": {
            //width: 100,
            transform: 'rotate(0deg)'
        },
        "1": {
            // width: 200,
            transform: 'rotate(360deg)'
        }
    });
    animationButton.setHoverStyle({
        animationName: 'rotate',
        animationDuration: 1000,
        animationIterationCount: Infinity,
    })
    console.log("animationButtonId", animationButton.getId());
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

function main() {
    runWorker();
    createSystemTray();
    console.log("begin create frame");
    const frame = new Frame();
    frame.setTitle("LentoDemo");
    frame.bindResize((e) => {
        console.log("frame resized", e);
    })
    console.log("frame created", frame);

    typeface_create("auto-mono", {
        family: "monospace",
        weight: "bold",
    })

    const container = new ScrollElement();
    container.setStyle({
        background: "#2a2a2a",
        color: "#FFF",
        padding: 5,
        gap: 5,
    })

    container.addChild(createAddChildrenButton(container));
    container.addChild(createAnimationButton());
    container.addChild(createLabel("测试test"));
    container.addChild(createParagraph());
    container.addChild(createTextEdit());
    container.addChild(createEntry());
    container.addChild(createCenterElement());
    batchCreateLabels(container);
    frame.setBody(container);
}

main();
