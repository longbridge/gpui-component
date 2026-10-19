import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import * as GPUI from "nsh/ui/gpui";
const Slot=nativeComponent("__nsh.Slot",{props:["__nshSlot"]});
const Button = nativeComponent("gpui-component.Button",{props:["label"],events:["press"]});
const Probe = nativeComponent("verify.probe",{props:["snapshot"]});
const names = ["Markdown","Html","CodeBlock","Diff","LineChart","AreaChart","BarChart","PieChart","RadarChart","CandlestickChart","SankeyChart","Plot","Image","SpeechButton","MarkdownSlots","PlotShapes","InlineMarkdownSlots","TextView","SpeechWaveform"];
const controls = names.map(name=>GPUI[(name==="MarkdownSlots"||name==="InlineMarkdownSlots")?"Markdown":name==="PlotShapes"?"Plot":name]);
mount(()=>{
  const index=signal(0);const error=signal(null);const line=signal(null);const slotClicks=signal(0);
  return [
    Button({label:"Increment",style:{width:140,height:36}}),
    Button({label:"Next content",onPress:()=>index.update(i=>(i+1)%names.length),style:{width:140,height:36}}),
    ()=>{
      const name=names[index()];
      const props={style:{width:550,height:360}};
      if(["TextView","Markdown","Html","CodeBlock","MarkdownSlots","InlineMarkdownSlots"].includes(name)){
        props.text=name==="MarkdownSlots" ? "Slot body" : name==="InlineMarkdownSlots" ? "Before **slot** after" : name==="Html" ? "<h2>Native HTML</h2><p>Rich <strong>content</strong></p>" : name==="CodeBlock" ? "const answer = 42;\nconsole.log(answer);" : "## Native Markdown\n\n- Retained text\n- **Formatted** content";
        if(name==="CodeBlock")props.language="javascript";
        if(name==="MarkdownSlots")props.blockSlots=[{source:"Slot body",slot:"body",text:"Slot action"}];
        if(name==="InlineMarkdownSlots")props.inlineSlots=[{source:"**slot**",slot:"inline",text:"Inline slot action"}];
      }else if(name==="Diff"){
        props.patch="--- a/demo.txt\n+++ b/demo.txt\n@@ -1 +1 @@\n-before\n+after\n";
        props.onLineClick=value=>line.set(value);
      }else if(name.endsWith("Chart")){
        props.data=name==="CandlestickChart" ? [{label:"Mon",open:10,high:16,low:8,close:14},{label:"Tue",open:14,high:18,low:11,close:12}] : [{label:"A",value:2},{label:"B",value:5},{label:"C",value:3}];
        props.appear=false;
        if(name==="SankeyChart")props.links=[{source:0,target:1,value:2},{source:1,target:2,value:1}];
      }else if(name==="Plot"||name==="PlotShapes"){
        props.series=name==="PlotShapes" ? [{type:"bar",points:[["A",2],["B",5],["C",3]],color:"chart2"},{type:"pie",values:[1,2,3],outerRadius:54,innerRadius:24,colors:["chart1","chart3","chart5"]}] : [{type:"line",points:[[0,0.2],[0.5,0.8],[1,0.4]]},{type:"area",points:[[0,0.1],[0.5,0.4],[1,0.2]]}];
        if(name==="PlotShapes"){
          props.scales={x:{type:"band",domain:["A","B","C"]},y:{domain:[0,6]}};
          props.margins={left:30,bottom:24,top:12,right:12};
          props.axes={x:{labels:[{label:"A",value:"A"},{label:"B",value:"B"},{label:"C",value:"C"}]},y:{side:"start",labels:[{label:"0",value:0},{label:"3",value:3},{label:"6",value:6}]}};
          props.grid={y:[0,3,6],dashes:[3,3]};
        }
      }else if(name==="Image")props.src="icons/check.svg";
      else props.showWhenUnsupported=true;
      if(name!=="Image")props.onError=value=>error.set(value);
      const node=controls[index()](props);if(names[index()]==="MarkdownSlots"){const slot=Slot({__nshSlot:"body"});insert(slot,Button({label:"Markdown slot action",style:{width:240,height:36},onPress:()=>slotClicks.update(v=>v+1)}));insert(node,slot);}if(names[index()]==="InlineMarkdownSlots"){const slot=Slot({__nshSlot:"inline"});insert(slot,Button({label:"Inline slot action",style:{width:180,height:36},onPress:()=>slotClicks.update(v=>v+1)}));insert(node,slot);}return node;},
    Probe({get snapshot(){return {fixture:"content",index:index(),name:names[index()],error:error(),line:line(),slotClicks:slotClicks()};},style:{width:0,height:0}}),
  ];
});
