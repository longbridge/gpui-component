import { View } from "gpui";
import {
  createVirtualListStory,
  renderVirtualListStory,
} from "../stories/virtual_list.js";

export default class VirtualListStoryFixture extends View {
  init() {
    this.story = createVirtualListStory();
  }

  render(cx) {
    return renderVirtualListStory(this.story, cx);
  }
}
