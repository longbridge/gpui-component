import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { Button, Calendar, DatePicker, TimeField } from "nsh/ui/gpui";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"], events: ["command"] });
mount(() => {
  const calendar = signal("2026-10-18");
  const range = signal(false);
  const navigation = signal(null);
  let calendarRef, pickerRef, timeRef;
  const veto = signal(false), timeNative = signal(null), dateNative = signal(null);
  const picked = signal("2026-10-18");
  const time = signal("09:30:15");
  const precision = signal("second");
  const cycle = signal("h23");
  const changes = signal(0);
  const error = signal(null);
  return [
    Button({ label: "Increment", style: { width: 220, height: 36 } }),
    Calendar({ get range() { return range(); }, ref: node => calendarRef = node, onSnapshot: value => navigation.set(value), get value() { return calendar(); }, disabledDates: ["2026-10-20"], onChange: value => { if(!veto()) calendar.set(value); changes.update(v => v + 1); }, onError: value => error.set(value), style: { width: 340, height: 340 } }),
    DatePicker({ ref: node => pickerRef = node, onSnapshot: value => dateNative.set(value), get range() { return range(); }, get value() { return picked(); }, placeholder: "Choose date", cleanable: true, get presets() { return range() ? [{ label: "November range", value: ["2026-11-14", "2026-11-16"] }, { label: "December range", value: ["2026-12-01", "2026-12-03"] }] : [{ label: "Next week", value: "2026-10-25" }]; }, onChange: value => { if(!veto()) picked.set(value); changes.update(v => v + 1); }, onError: value => error.set(value), style: { width: 260, height: 36 } }),
    TimeField({ ref: node => timeRef = node, onSnapshot: value => timeNative.set(value), get value() { return time(); }, get precision() { return precision(); }, get hourCycle() { return cycle(); }, onChange: value => { if(!veto()) time.set(value); changes.update(v => v + 1); }, onError: value => error.set(value), style: { width: 220, height: 36 } }),
    Probe({ get snapshot() { return { fixture: "datetime", veto: veto(), timeNative: timeNative(), dateNative: dateNative(), calendar: calendar(), picked: picked(), time: time(), changes: changes(), navigation: navigation(), range: range(), cycle: cycle(), error: error() }; }, onCommand: command => { if(command === "veto-calendar") { veto.set(true); calendarRef.command("activateDate", "2026-11-15"); calendarRef.command("activateDate", "2026-11-17"); } if(command === "calendar-snapshot") calendarRef.command("snapshot"); if(command === "time-snapshot") timeRef.command("snapshot"); if(command === "picker-snapshot") pickerRef.command("snapshot"); if(command === "twelve") { precision.set("minute"); cycle.set("h12"); } if(command === "range") { range.set(true); calendar.set(["2026-11-10", null]); picked.set(["2026-11-10", null]); } if(command === "navigate") { calendarRef.command("nextMonth"); } if(command === "reset") { calendar.set("2026-10-22"); picked.set("2026-10-23"); time.set("14:45:30"); } }, style: {width: 0, height: 0} }),
  ];
});
