import { mount, signal, View, Text } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
const Button = nativeComponent("gpui-component.Button", {props:["label"],events:["press"]});
const Input = nativeComponent("gpui-component.Input", {props:["value","accessibilityLabel"],events:["change"]});
const RawHost = nativeComponent("verify.host", {props:[]});
const Slot = nativeComponent("__nsh.Slot", {props:["__nshSlot"]});
const Probe = nativeComponent("verify.probe", {props:["snapshot"],events:["advance"]});
mount(() => {
 const count=signal(0), name=signal(""), show=signal(true);
 const increment=()=>count.update(v=>v+1);
 const host=RawHost({style:{width:440,height:220}});
 const header=Slot({__nshSlot:"header"});
 insert(header, Button({label:"Slot increment",onPress:increment}));
 insert(host,header);
 const plain = View({style:{width:240,height:42}, accessibilityLabel:"NSH view increment", accessibilityRole:"button",onPress:increment});
 insert(plain, Text({children:"Plain NSH subtree"}));
 insert(host, plain);
 insert(host,()=>show() ? [
   Button({label:"Child increment",onPress:increment}),
   Input({get value(){return name();},accessibilityLabel:"Child input",onChange:v=>name.set(v)}),
 ]:[]);
 return [host,
 Button({label:"Unmount children",style:{width:200,height:36},onPress:()=>show.set(false)}),
 Probe({onAdvance:increment,get snapshot(){return {count:count(),name:name(),show:show()};},style:{width:0,height:0}})];
});
