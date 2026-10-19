import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { Input as TokenInput, Editor, Textarea, NumberInput, OtpInput as Otp, InputGroup as Group } from "nsh/ui/gpui";
const Button=nativeComponent("gpui-component.Button",{props:["label"],events:["press"]});
const Probe=nativeComponent("verify.probe",{props:["snapshot"],events:[]});
mount(()=>{
 const groupMode=signal("textarea"),groupValue=signal("");
 const count=signal(0),editor=signal("abc"),textarea=signal(""),number=signal("2"),otp=signal(""),complete=signal(0),focused=signal(""),error=signal(null),language=signal("rust"),services=signal(false),requests=signal([]),recipient=signal("To @Ada"),content=signal({text:"To @Ada",tokens:[{id:"ada",text:"@Ada",label:"Ada",start:3,end:7}]});
 let otpRef,editorRef;
 const failure=(value)=>error.set(value);
 return [
 Button({label:"Increment",style:{width:180,height:32},onPress:()=>count.update(n=>n+1)}),
 Editor({ref:r=>editorRef=r,get value(){return editor()},accessibilityLabel:"Source code",height:130,get language(){return language()},get languageServices(){return services()?{completion:true}:{}},onRequest:request=>{requests.update(items=>[...items,request.method]);Promise.resolve().then(()=>editorRef.command("resolveRequest",{id:request.id,result:request.method==="textDocument/completion"?[{label:"print",textEdit:{range:{start:{line:0,character:0},end:{line:0,character:request.params.offset}},newText:"print"}}]:[]}));},style:{width:400,height:130},onChange:v=>editor.set(v),onFocus:()=>focused.set("editor"),onError:failure}),
 Textarea({get value(){return textarea()},accessibilityLabel:"Notes",height:70,rows:2,style:{width:400,height:70},onChange:v=>textarea.set(v),onError:failure}),
 NumberInput({get value(){return number()},accessibilityLabel:"Quantity",step:2,min:0,max:10,style:{width:200,height:36},onChange:v=>number.set(v),onFocus:()=>focused.set("number"),onError:failure}),
 Otp({ref:r=>otpRef=r,length:4,get value(){return otp()},style:{width:240,height:40},onChange:v=>otp.set(v),onComplete:()=>complete.update(n=>n+1),onFocus:()=>focused.set("otp"),onError:failure}),
 Button({label:"Enable language services",style:{width:180,height:32},onPress:()=>services.set(true)}),
 TokenInput({get content(){return content()},accessibilityLabel:"Recipient",style:{width:400,height:36},onChange:v=>recipient.set(v),onContentChange:v=>content.set(v),onError:failure}),
 Group({get mode(){return groupMode()},get value(){return groupValue()},height:64,accessibilityLabel:"Grouped notes",style:{width:400,height:70},onChange:v=>groupValue.set(v),onError:failure}),
 Button({label:"Switch group mode",style:{width:180,height:32},onPress:()=>{groupMode.set("input");groupValue.set("Single");}}),
 Button({label:"Focus code",style:{width:180,height:32},onPress:()=>otpRef.command("focus",null)}),
 Button({label:"Select editor text",style:{width:180,height:32},onPress:()=>{editorRef.command("focus",null);editorRef.command("select",{start:0,end:2});}}),
 Button({label:"Change language",style:{width:180,height:32},onPress:()=>language.set("javascript")}),
 Probe({get snapshot(){return{fixture:"inputs",count:count(),editor:editor(),textarea:textarea(),number:number(),otp:otp(),complete:complete(),focused:focused(),error:error(),language:language(),services:services(),requests:requests(),recipient:recipient(),content:content(),groupMode:groupMode(),groupValue:groupValue()}},style:{width:0,height:0}})
 ];
});
