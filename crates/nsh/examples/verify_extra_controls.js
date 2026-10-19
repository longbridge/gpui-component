import { mount, signal, View, Text } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import { Button, Stepper, ColorPicker, ColorSelect } from "nsh/ui/gpui";
const Probe = nativeComponent("verify.probe", {props:["snapshot"],events:["command"]});
mount(() => {
  const slotLayout = signal(null);
  const step = signal(0), stepChanges = signal(0), slotClicks = signal(0), allDisabled = signal(false);
  const value = signal("#336699"), background = signal(null), changes = signal(0), selectChanges = signal(0), open = signal(false), opens = signal(0), tab = signal(0), tabs = signal(0), snapshot = signal(null), precise = signal(null), error = signal(null);
  let picker, select;
  const veto = signal(false);
  const slot=View({onLayout: bounds=>slotLayout.set(bounds)});
  insert(slot,Button({label:"Step slot action",onPress:()=>slotClicks.update(v=>v+1),style:{width:180,height:32}}));
  insert(slot,Text({get children(){return `Slot clicks: ${slotClicks()}`;}}));
  const stepper = Stepper({items:[{id:"start",label:"Start"},{id:"review",label:"Review"},{id:"finish",label:"Finish",disabled:true}],slots:{"step:start":slot},get selectedIndex(){return step();},get disabled(){return allDisabled();},onChange: index=>{step.set(index);stepChanges.update(v=>v+1);},onError:e=>error.set(e),style:{width:600,height:140}});
  return [
    Button({label:"Increment",style:{width:220,height:36}}),stepper,
    ColorSelect({ref:n=>select=n,get value(){return background();},placeholder:"Choose background",accessibilityLabel:"Background",featuredColors:["#0000FF","#ABCDEF"],onChange:v=>{if(!veto()) background.set(v);selectChanges.update(n=>n+1);},onSnapshot:v=>snapshot.set(v),onError:e=>error.set(e),style:{width:280,height:36}}),
    View({style:{height:180,width:1}}),
    ColorPicker({ref:n=>picker=n,get value(){return value();},label:"Foreground",accessibilityLabel:"Text color",featuredColors:["#ff0000","#00ff00","#ff0000"],anchor:"bottom-left",get open(){return open();},get activeTab(){return tab();},onChange:v=>{if(!veto()) value.set(v);changes.update(n=>n+1);},onHslaChange:v=>precise.set(v),onOpenChange:v=>{open.set(v);opens.update(n=>n+1);},onActiveTabChange:v=>{tab.set(v);tabs.update(n=>n+1);},onSnapshot:v=>snapshot.set(v),onError:e=>error.set(e),style:{width:280,height:36}}),
    Probe({get snapshot(){return {fixture:"extra_controls",slotLayout:slotLayout(),veto:veto(),step:step(),stepChanges:stepChanges(),slotClicks:slotClicks(),allDisabled:allDisabled(),value:value(),background:background(),changes:changes(),selectChanges:selectChanges(),open:open(),opens:opens(),tab:tab(),tabs:tabs(),native:snapshot(),precise:precise(),error:error()};},onCommand:command=>{
      switch(command){
        case "veto-picker":veto.set(true);picker.command("updateColor","#FF00FF");break;
        case "veto-select":select.command("selectColor","#FEDCBA");break;
        case "focus-picker":picker.command("focus");break;
        case "disable-steps":allDisabled.set(true);break;
        case "reset":value.set("#FF000080");background.set("#FFFF00");break;
        case "snapshot":picker.command("snapshot");break;
        case "preview":picker.command("previewColor","#00FF00");break;
        case "clear-preview":picker.command("clearPreview");break;
        case "hsla":picker.command("updateColor",{h:0.4321,s:0.6789,l:0.3456,a:0.7654});break;
        case "open-tab":open.set(true);tab.set(1);break;
        case "close":open.set(false);break;
        case "commit-hex":picker.command("commitHex","#1234");break;
        case "select-snapshot":select.command("snapshot");break;
      }
    },style:{width:0,height:0}}),
  ];
});
