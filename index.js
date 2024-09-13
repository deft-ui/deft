function main() {
    console.log("begin create frame");
    const frame = new Frame();
    frame.setTitle("LentoDemo");
    console.log("frame created", frame);

    const container = new ScrollElement();
    container.setStyle({
        background: "#2a2a2a",
        color: "#FFF",
    })
    container.bindMouseMove(e => {
        console.log("mouse move", e);
    })

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
    // const button = new ButtonElement();
    // button.setTitle("Add children");
    // button.bindClick(() => {
    //     for (let i = 0; i < 1000; i++) {
    //         const lb = new LabelElement();
    //         lb.setText("label" + i);
    //         container.addChild(lb);
    //     }
    //     console.log("done");
    // })
    //
    // const textEdit = new TextEditElement();
    //
    // //textEdit.setAlign("center")
    // textEdit.setText("TextEdit 测试test");
    //
    // textEdit.setStyle({
    //     "height": 100,
    //     "width": 100,
    //     "background": "#ccc",
    //     // "border": "1 #ccc"
    //     // "minWidth": 600,
    // });
    //
    // container.addChild(textEdit);
    // container.addChild(button);
    //
    // //container.removeChild(label2);
    //
    // console.log("setBody")
    container.addChild(label);
    frame.setBody(container);
}

main();
