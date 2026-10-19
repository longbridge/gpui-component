import { Button, Carousel, Questionnaire, MessageScroller as Scroller, Sidebar, VirtualList } from "nsh/ui/gpui";
import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
const Slot=nativeComponent("__nsh.Slot",{props:["__nshSlot"]});
const Probe=nativeComponent("verify.probe",{props:["snapshot"]});
function slot(parent,name,child){const node=Slot({__nshSlot:name});insert(node,child);insert(parent,node);}
mount(()=>{const phase=signal("carousel");const selected=signal(0);const survey=signal(null);const submitted=signal(null);const error=signal(null);const clicks=signal(0);const destination=signal("home");return [Button({label:"Increment",style:{width:200,height:32}}),...[["Carousel","carousel"],["Questionnaire","survey"],["Messages","messages"],["Navigation","navigation"],["Virtual list","virtual"]].map(([label,key])=>Button({label,style:{width:200,height:32},onPress:()=>phase.set(key)})),()=>{
if(phase()==="carousel"){const node=Carousel({items:["slide-a","slide-b"],get value(){return selected();},accessibilityLabel:"Project slides",style:{width:350,height:220},onChange:value=>selected.set(value)});slot(node,"slide-a",Button({label:"Slide A"}));slot(node,"slide-b",Button({label:"Slide B"}));return node;}
if(phase()==="survey"){return Questionnaire({items:[{id:"answer",label:"Pick an answer",required:true,choices:[{value:"a",label:"First answer"},{value:"b",label:"Second answer"}]}],style:{width:350,height:350},onChange:value=>survey.set(value),onSubmit:value=>submitted.set(value),onError:value=>error.set(value)});}
if(phase()==="messages"){const ids=Array.from({length:20},(_,i)=>`message-${i}`);const node=Scroller({items:ids,style:{width:350,height:340}});for(const id of ids)slot(node,id,Button({label:id,onPress:()=>clicks.update(value=>value+1)}));return node;}
if(phase()==="virtual"){const ids=["virtual-row-a","virtual-row-b"];const node=VirtualList({items:ids,sizes:[[350,40],[350,40]],style:{width:350,height:120}});for(const id of ids)slot(node,id,Button({label:id,onPress:()=>clicks.update(value=>value+1)}));return node;}
return Sidebar({items:[{id:"home",label:"Home"},{id:"projects",label:"Projects"}],get value(){return destination();},onChange:value=>destination.set(value),style:{width:250,height:340}});
},Probe({get snapshot(){return {fixture:"layout-systems",phase:phase(),selected:selected(),survey:survey(),submitted:submitted(),error:error(),clicks:clicks(),destination:destination()};},style:{width:1,height:1}})];});
