import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import { Button, Popover, HoverCard as Hover, Tooltip, DropdownMenu as Menu, ContextMenu, NativeMenu } from "nsh/ui/gpui";
const Slot = nativeComponent("__nsh.Slot", {props:["__nshSlot"]});
const Probe = nativeComponent("verify.probe", {props:["snapshot"]});
function slot(parent,name,child) { const node=Slot({__nshSlot:name}); insert(node,child); insert(parent,node); return node; }
const frame={width:220,height:36};
mount(()=>{
 const open=signal(false), hover=signal(false), presses=signal(0), selected=signal(null), menuOpen=signal(false);
 const changes=signal(0), selections=signal(0), custom=signal(false), roundtrip=signal(0), validationError=signal(null), error=signal(null);
 const select=value=>{selected.set(value);selections.update(n=>n+1);};
 let popoverRef, switchRef;
 const popover=Popover({ref:node=>popoverRef=node,label:"Open popover",get open(){return open();},onOpenChange:value=>{changes.update(n=>n+1);open.set(value);},onError:value=>error.set(value),style:frame});
 slot(popover,"content",Button({label:"Popover item",onPress:()=>{presses.update(n=>n+1);open.set(false);}}));
 const card=Hover({label:"Hover card",openDelay:50,closeDelay:50,onOpenChange:value=>hover.set(value),onError:value=>error.set(value),style:frame});
 slot(card,"content",Button({label:"Hover item",onPress:()=>presses.update(n=>n+1)}));
 const tooltip=Tooltip({text:"Native tooltip",style:frame});
 slot(tooltip,"trigger",Button({label:"Tooltip trigger"}));
 slot(tooltip,"content",Button({label:"Tooltip live content",style:{width:180,height:32}}));
 const items=[{id:"copy",label:"Copy item"},{id:"disabled",label:"Unavailable",disabled:true,checked:true},{separator:true},{label:"More",items:[{id:"paste",label:"Paste item"}]},{label:"Disabled submenu",disabled:true,items:[{id:"blocked",label:"Blocked item"}]}];
 const context=ContextMenu({items,onSelect:select,onError:value=>error.set(value),style:frame});
 slot(context,"trigger",Button({label:"Context trigger"}));
 const menu=Menu({label:"Open menu",items,onSelect:select,onOpenChange:value=>menuOpen.set(value),onError:value=>error.set(value),style:frame});
 insert(menu,()=>{
   if(!custom()) return null;
   const node=Slot({__nshSlot:"trigger"});
   insert(node,Button({label:"Custom menu trigger"}));
   return node;
 });
 const nativeItems=signal(items);
 const native=NativeMenu({label:"Native commands",get items(){return nativeItems();},onSelect:select,onError:value=>validationError.set(value),style:frame});
 slot(native,"trigger",Button({label:"Native trigger"}));
 return [Button({label:"Increment",onPress:()=>roundtrip.update(n=>n+1),style:frame}),popover,card,tooltip,
   Button({ref:node=>switchRef=node,label:"Switch trigger",onPress:()=>{custom.update(value=>!value);switchRef.command("focus");},style:frame}),menu,context,native,
   Button({label:"Focus popover",onPress:()=>popoverRef.command("focus"),style:frame}),
   Button({label:"Reject invalid items",onPress:()=>nativeItems.set([{id:"same",label:"One"},{id:"same",label:"Two"}]),style:frame}),
   Probe({get snapshot(){return {fixture:"surfaces",open:open(),hover:hover(),presses:presses(),selected:selected(),menuOpen:menuOpen(),changes:changes(),selections:selections(),custom:custom(),roundtrip:roundtrip(),validationError:validationError(),error:error()};},style:{width:0,height:0}})];
});
