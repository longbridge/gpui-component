// An NSH app the `example-nsh` example embeds in a GPUI Kit window. Layout
// and text are NSH's; the controls are GPUI Base's, which the host registers
// as native components (`gpui-nsh`). GPUI Base owns their behavior and draws
// nothing of its own, so the class strings below are the whole look: GPUI's
// style methods plus `bg_` / `text_` / `border_` with GPUI Base's color
// tokens, which follow the host's theme.

import { mount, signal, Text, View } from "nsh";
import {
  Button,
  Checkbox,
  Input,
  Progress,
  RadioGroup,
  Select,
  Slider,
  Switch,
  Tabs,
} from "../../../crates/nsh/types/gpui-base";

/** NSH's text is black by default; `dark:` follows the host's mode. */
const text = "text-zinc-900 dark:text-zinc-100";

const tab = signal(0);
const name = signal("Ada");
const alerts = signal(true);
const digest = signal(false);
const plan = signal(0);
const volume = signal(40);
const region = signal("Europe");
const storage = signal(30);

const button = "px_3 py_1p5 rounded_md border_1 border_border bg_background text_sm";
const field = "w_full px_2 py_1p5 rounded_md border_1 border_input bg_background text_sm";

function Row(props: { title: string; children: unknown }) {
  return (
    <View style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
      <Text class={text} style={{ width: 96 }}>
        {props.title}
      </Text>
      <View style={{ flex: 1, alignItems: "flex-start" }}>{props.children}</View>
    </View>
  );
}

function Profile() {
  return (
    <View style={{ gap: 12 }}>
      <Row title="Name">
        <View style={{ alignSelf: "stretch" }}>
          <Input
            value={name()}
            placeholder="Your name"
            onChange={(e) => name.set(e.value)}
            rootClass={field}
            focusedClass="border_ring"
          />
        </View>
      </Row>
      <Row title="Region">
        <Select
          items={["Asia", "Europe", "Americas"]}
          value={region()}
          onChange={(e) => region.set(e.value)}
          rootClass="w_48"
          triggerClass={`${field} flex justify_between`}
          contentClass="w_48 mt_1 p_1 rounded_md border_1 border_border bg_background"
          itemClass="w_full justify_start px_2 py_1 rounded_sm text_sm"
          selectedItemClass="bg_accent text_accent_foreground"
        />
      </Row>
      <Text class="text-zinc-500 dark:text-zinc-400">{`Hello, ${name() || "stranger"} from ${region()}.`}</Text>
    </View>
  );
}

function Notifications() {
  return (
    <View style={{ gap: 12 }}>
      <Row title="Alerts">
        <Switch
          label="Send alerts"
          checked={alerts()}
          onChange={(e) => alerts.set(e.checked)}
          rootClass="flex items_center gap_2 text_sm"
          trackClass="flex w_9 h_5 p_0p5 rounded_full bg_muted"
          trackCheckedClass="justify_end bg_primary"
          thumbClass="size_4 rounded_full bg_background"
        />
      </Row>
      <Row title="Digest">
        <Checkbox
          label="Weekly digest"
          checked={digest()}
          disabled={!alerts()}
          onChange={(e) => digest.set(e.checked)}
          rootClass="flex items_center gap_2 text_sm"
          disabledClass="text_muted_foreground"
          indicatorClass="size_4 rounded_sm border_1 border_primary"
          indicatorDisabledClass="border_muted_foreground"
          indicatorCheckedClass="bg_primary"
        />
      </Row>
      <Row title="Plan">
        <RadioGroup
          items={["Monthly", "Yearly"]}
          value={plan()}
          onChange={(e) => plan.set(e.value)}
          rootClass="flex gap_4"
          axis="horizontal"
          radioClass="px_2 py_1 rounded_md border_1 border_border text_sm"
          checkedClass="bg_primary text_primary_foreground"
        />
      </Row>
      <Row title="Volume">
        <View style={{ alignSelf: "stretch" }}>
          <Slider
            value={volume()}
            onChange={(e) => volume.set(e.value)}
            rootClass="w_full h_5"
            trackClass="flex items_center w_full h_5"
            indicatorClass="w_full h_1p5 rounded_full bg_muted"
            rangeClass="h_full rounded_full bg_primary"
            thumbClass="size_4 rounded_full border_2 border_primary bg_background"
          />
        </View>
      </Row>
    </View>
  );
}

function Storage() {
  return (
    <View style={{ gap: 12 }}>
      <Row title="Used">
        <View style={{ alignSelf: "stretch" }}>
          <Progress
            value={storage()}
            rootClass="w_full"
            trackClass="w_full h_2 rounded_full bg_muted"
            indicatorClass="h_full rounded_full bg_primary"
          />
        </View>
      </Row>
      <View style={{ flexDirection: "row", gap: 8 }}>
        <Button label="Use more" rootClass={button} onClick={() => storage.update((p) => Math.min(100, p + 10))} />
        <Button label="Reset" rootClass={button} onClick={() => storage.set(30)} />
      </View>
    </View>
  );
}

function App() {
  return (
    <View style={{ flex: 1, padding: 24, gap: 20 }}>
      <Tabs
        items={["Profile", "Notifications", "Storage"]}
        value={tab()}
        onChange={(e) => tab.set(e.value)}
        rootClass="flex gap_1 p_1 rounded_lg bg_muted"
        tabClass="px_3 py_1 rounded_md text_sm text_muted_foreground"
        selectedClass="bg_background text_foreground"
      />
      {tab() === 0 ? <Profile /> : tab() === 1 ? <Notifications /> : <Storage />}
    </View>
  );
}

mount(App);
