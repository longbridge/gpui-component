import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { AlertDialog as Alert } from "nsh/ui/gpui";
const Button=nativeComponent("gpui-component.Button",{props:["label"],events:["press"]});
const Probe=nativeComponent("verify.probe",{props:["snapshot"],events:["command"]});
mount(()=>{
 const veto=signal(false), open=signal(false), allow=signal(false), confirms=signal(0), cancels=signal(0), closes=signal(0), closeButton=signal(true), error=signal(null);
 let opener;
 return [Button({label:"Increment",style:{width:220,height:36}}),
 Button({ref:value=>opener=value,label:"Open alert",style:{width:220,height:36},onPress:()=>{opener.command("focus",null);open.set(true);}}),
 Alert({get open(){return open();},title:"Confirm change",description:"An alert keeps its native header and footer",showCancel:true,
  get closeButton(){return closeButton()?true:undefined;},get closeOnConfirm(){return allow();},
  onConfirm:()=>confirms.update(n=>n+1),onCancel:()=>cancels.update(n=>n+1),
  onOpenChange:value=>{if(!veto())open.set(value);closes.update(n=>n+1);},onError:value=>error.set(value)}),
 Probe({get snapshot(){return {fixture:"alert",veto:veto(),open:open(),allow:allow(),confirms:confirms(),cancels:cancels(),closes:closes(),closeButton:closeButton(),error:error()};},
 onCommand:value=>{if(value==="veto-close")veto.set(true);if(value==="accept-close")veto.set(false);if(value==="allow")allow.set(true);if(value==="remove-close")closeButton.set(false);},style:{width:0,height:0}})];
});
