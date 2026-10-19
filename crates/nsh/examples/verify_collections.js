import { mount, signal, Text } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import {Button,List,DataTable as Table,Tree,Combobox,Command,Pagination,Table as BasicTable,TableHeader as Header,TableBody as Body,TableRow as Row,TableHead as Head,TableCell as Cell,ListItem} from "nsh/ui/gpui";
const Slot=nativeComponent("__nsh.Slot",{props:["__nshSlot"]});
function child(parent, children){insert(parent,children);return parent;}
const Probe=nativeComponent("verify.probe",{props:["snapshot"],events:[]});
mount(()=>{
 const phase=signal("list"),count=signal(0),error=signal(null),focus=signal(""),listId=signal("a"),listConfirm=signal(null),query=signal(""),searchable=signal(true),selections=signal(0),reversed=signal(false);
 const tableSelection=signal(null),sort=signal(null),exported=signal(null),treeId=signal(null),treeConfirm=signal(null),expanded=signal(null),values=signal([]),comboConfirm=signal(null),commandId=signal(null),commandConfirm=signal(null),commandQuery=signal(""),page=signal(1),cellCount=signal(0),partCount=signal(0);
 let ref;
 let beforeRef,afterRef;
 const commandSearchable=signal(true),commandConfirms=signal(0);
 const btn=(label,onPress)=>Button({label,style:{width:200,height:32},onPress});
 const failure=value=>error.set(value);
 const listItems=[{id:"a",label:"Alpha"},{id:"b",label:"Beta"},{id:"c",label:"Disabled",disabled:true}];
 return [
 btn("Increment",()=>count.update(n=>n+1)),
 btn("Focus collection",()=>{ref.command("focus",null);focus.set(phase())}),
 btn("Reorder items",()=>reversed.set(!reversed())),
 btn("Show table",()=>phase.set("table")),btn("Show tree",()=>phase.set("tree")),btn("Show command",()=>phase.set("command")),btn("Show combobox",()=>phase.set("combobox")),btn("Show pagination",()=>phase.set("pagination")),btn("Show table parts",()=>phase.set("parts")),
 Button({ref:r=>beforeRef=r,label:"Before collection",style:{width:200,height:32},onPress:()=>beforeRef.command("focus",null)}),
 ()=>{
  if(phase()==="list")return List({ref:r=>ref=r,get items(){return reversed()?[...listItems].reverse():listItems},get selectedId(){return listId()},get searchable(){return searchable()},get query(){return query()},placeholder:"Find items",style:{width:440,height:210},onSelectionChange:id=>{listId.set(id);selections.update(n=>n+1)},onConfirm:value=>listConfirm.set(value),onQueryChange:value=>{query.set(value);if(value==="Alpha")searchable.set(false)},onError:failure});
  if(phase()==="table"){
   const node=Table({ref:r=>ref=r,columns:[{key:"name",label:"Name",sortable:true,width:180},{key:"score",label:"Score",width:120}],rows:[{id:"a",cells:{name:"Alpha",score:20}},{id:"b",cells:{name:"Beta",score:10}}],get selection(){return tableSelection()},style:{width:440,height:210},onSelectionChange:value=>tableSelection.set(value),onSort:value=>sort.set(value),onExport:value=>exported.set(value),onError:failure});
   const slot=Slot({__nshSlot:"cell:b:name"});insert(slot,btn("Cell action",()=>cellCount.update(n=>n+1)));insert(node,slot);return node;
  }
  if(phase()==="tree")return Tree({ref:r=>ref=r,items:[{id:"root",label:"Folder",children:[{id:"leaf",label:"Document"}]},{id:"other",label:"Other"}],get selectedId(){return treeId()},style:{width:440,height:210},onSelectionChange:id=>treeId.set(id),onExpandedChange:value=>expanded.set(value),onConfirm:value=>treeConfirm.set(value),onError:failure});
  if(phase()==="command")return Command({ref:r=>ref=r,items:[{id:"a",label:"Alpha action"},{id:"b",label:"Beta action",group:"Tools",keywords:["bravo"]}],get selectedId(){return commandId()},get query(){return commandQuery()},get searchable(){return commandSearchable()},style:{width:440,height:210},onSelectionChange:id=>commandId.set(id),onQueryChange:value=>{commandQuery.set(value);if(value==="bravo")commandSearchable.set(false)},onConfirm:value=>{commandConfirm.set(value);commandConfirms.update(n=>n+1)},onError:failure});
  if(phase()==="combobox")return Combobox({ref:r=>ref=r,groups:[{label:"Tags",items:[{value:"a",label:"Alpha tag"},{value:"b",label:"Beta tag"}]}],get values(){return values()},multiple:true,placeholder:"Choose tags",style:{width:300,height:36},onChange:value=>values.set(value),onConfirm:value=>comboConfirm.set(value),onError:failure});
  if(phase()==="parts"){
   const text=(content)=>Text({children:content});
   const head=child(Header({style:{width:400,height:40}}),child(Row({style:{width:400,height:40}}),[child(Head({style:{width:200,height:40}}),text("Name")),child(Head({style:{width:200,height:40}}),text("Action"))]));
   const row=child(Row({style:{width:400,height:44}}),[child(Cell({style:{width:200,height:44}}),text("Alpha")),child(Cell({style:{width:200,height:44}}),btn("Compound cell",()=>partCount.update(n=>n+1)))]);
   return [child(BasicTable({style:{width:400,height:100}}),[head,child(Body({style:{width:400,height:44}}),row)]),ListItem({label:"Standalone list item",style:{width:400,height:36},onPress:()=>partCount.update(n=>n+1)})];
  }
  return Pagination({get page(){return page()},totalPages:4,style:{width:440,height:48},onChange:value=>page.set(value),onError:failure});
 },
 Button({ref:r=>afterRef=r,label:"After collection",style:{width:200,height:32},onPress:()=>afterRef.command("focus",null)}),
 btn("Export table",()=>ref.command("export",null)),
 Probe({get snapshot(){return{fixture:"collections",phase:phase(),count:count(),error:error(),focus:focus(),listId:listId(),listConfirm:listConfirm(),query:query(),searchable:searchable(),selections:selections(),reversed:reversed(),tableSelection:tableSelection(),sort:sort(),exported:exported(),treeId:treeId(),treeConfirm:treeConfirm(),expanded:expanded(),values:values(),comboConfirm:comboConfirm(),commandId:commandId(),commandConfirm:commandConfirm(),commandQuery:commandQuery(),commandSearchable:commandSearchable(),commandConfirms:commandConfirms(),page:page(),cellCount:cellCount(),partCount:partCount()}},style:{width:0,height:0}})
 ];
});
