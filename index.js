function main() {
    console.log("begin create frame");
    const frame = new Frame();
    frame.setTitle("LentoDemo");
    console.log("frame created", frame);

    const container = new ScrollElement();
    container.setStyle({
        background: "#2a2a2a",
        color: "#FFF",
        padding: 5,
    })
    // container.bindMouseMove(e => {
    //     console.log("mouse move", e);
    // })

    const label = new LabelElement();
    label.setAlign("center")
    label.setText("测试test");
    label.setStyle({
        "border-top": "#F00 3",
        "border-right": "#0F0 3",
        "border-bottom": "#00F 3",
        "border-left": "#0F0 3"
    });
    // label.bindMouseDown((detail) => {
    //     console.log("onClick111", detail);
    //     // label.setText(new Date().toString());
    // })
    // const label2 = new LabelElement();
    // label2.setAlign("center");
    // label2.setText("Label2");
    // container.addChild(label2);
    // container.addChild(label);
    //
    // const img = new ImageElement();
    // img.setSrc("img.png");
    // container.addChild(img);
    //
    const button = new ButtonElement();
    button.setTitle("Add children");
    button.bindClick(() => {
        const wrapper = new ContainerElement();
        wrapper.setStyle({
            flexDirection: 'row',
            flexWrap: 'wrap',
        })
        for (let i = 0; i < 200; i++) {
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
            wrapper.addChild(lb);
        }
        container.addChild(wrapper);
        console.log("done");
    });
    container.addChild(button);

    let animationButton = new ButtonElement();
    animationButton.setStyle({
        width: 100,
    });
    animationButton.setTitle("Animation");
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
    container.addChild(animationButton);


    const textEdit = new TextEditElement();

    //textEdit.setAlign("center")
    textEdit.setText("TextEdit 测试test");

    textEdit.setStyle({
        "height": 50,
        "width": 100,
        // "background": "#ccc",
        // "border": "1 #ccc"
        // "minWidth": 600,
    });

    container.addChild(textEdit);
    //
    // //container.removeChild(label2);
    //
    // console.log("setBody")
    container.addChild(label);
    frame.setBody(container);
}

main();
