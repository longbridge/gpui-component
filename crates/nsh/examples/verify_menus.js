import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import { Button, AppMenuBar as Bar, PopupMenu as Popup } from "nsh/ui/gpui";
const Slot = nativeComponent("__nsh.Slot",{props:["__nshSlot"]});
const Probe = nativeComponent("verify.probe",{props:["snapshot"],events:["command"]});
mount(()=>{
 const a=signal(true), b=signal(false), opened=signal(false), version=signal(0), chosen=signal([]), closed=signal(0), error=signal(null), clicks=signal(0);
 let startRef, openerRef;
 const choose=prefix=>value=>chosen.set([...chosen(),`${prefix}:${value}`]);
 const popup=Popup({get open(){return opened();},get items(){return [{id:"disabled",label:"Disabled popup item",disabled:true},{label:"Blocked submenu",disabled:true,items:[{id:"nested",label:"Hidden disabled action"}]},{id:"custom",label:"Custom popup item"},{id:"plain",label:`Popup action ${version()}`}];},onSelect:choose("popup"),onOpenChange:value=>{opened.set(value);closed.update(v=>v+1);},onError:error.set,style:{width:260,height:160}});
 const slot=Slot({__nshSlot:"item:custom"});
 insert(slot,Button({label:"Live custom menu row",onPress:()=>clicks.update(v=>v+1),style:{width:220,height:32}}));insert(popup,slot);
 return [
 Button({label:"Increment",ref:node=>startRef=node,style:{width:200,height:32}}),
 Button({label:"Open popup",ref:node=>openerRef=node,onPress:()=>opened.set(true),style:{width:200,height:32}}),
 ()=>a()?Bar({menus:[{name:"File",items:[{id:"run-a",label:"Run A"}]},{name:"Unavailable",disabled:true,items:[{id:"disabled-a",label:"Must stay hidden"}]}],onSelect:choose("a"),onError:error.set,style:{width:360,height:30}}):null,
 ()=>b()?Bar({menus:[{name:"Edit",items:[{id:"run-b",label:"Run B"}]}],onSelect:choose("b"),onError:error.set,style:{width:360,height:30}}):null,
 popup,
 Probe({get snapshot(){return {fixture:"menus",a:a(),b:b(),opened:opened(),chosen:chosen(),closed:closed(),clicks:clicks(),version:version(),error:error()};},onCommand:command=>{if(command==="focus-start")startRef.command("focus");if(command==="focus-open")openerRef.command("focus");if(command==="mount-b")b.set(true);if(command==="unmount-b")b.set(false);if(command==="unmount-a")a.set(false);if(command==="owner-close")opened.set(false);if(command==="update")version.update(v=>v+1);},style:{width:1,height:1}})
 ];
});
