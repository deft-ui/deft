function createLabel(text) {
    const label = new LabelElement();
    label.setText(text);
    return label;
}

const frame = new Frame();
frame.setTitle("LentoDemo");

const container = new ScrollElement();
container.setStyle({
    color: '#F9F9F9',
    justifyContent: 'center',
    alignContent: 'center',
})

container.addChild(createLabel("Hello,world!"));
container.addChild(createLabel("你好,世界!"));

frame.setBody(container);