import { View, div } from "gpui";
import { v_flex } from "gpui-base";
import {
  demoValue,
  initializeRegisteredExamples,
  registeredExamples,
} from "../stories/registered.js";

export default class InputStoryFixture extends View {
  init() {
    initializeRegisteredExamples();
    this.projectName = demoValue("input-project-name", null);
    this.projectName.on("change", (_event, cx) => cx.notify());
  }

  render() {
    const inputExample = registeredExamples("Input")[0];
    return v_flex()
      .size_full()
      .gap(16)
      .p(20)
      .child(
        div()
          .id("input-target")
          .w(420)
          .child(inputExample.element),
      )
      .child(div().child(`input:${this.projectName.value()}`));
  }
}
