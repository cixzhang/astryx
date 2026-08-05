import{i as e,s as t}from"./preload-helper-CT_b8DTk.js";import{t as n}from"./react-B7Te67-h.js";import{t as r}from"./jsx-runtime-DqZldVDK.js";import{t as i}from"./Text-CYfhD539.js";import{t as a}from"./Button-UlcpDuSp.js";import{t as o}from"./Button-GirzArYv.js";import{t as s}from"./Divider-D1M6lhgx.js";import{t as c}from"./Divider-Dai8fqTY.js";import{t as l}from"./CheckboxInput-Cw2AyaNh.js";import{t as u}from"./Heading-BGyAb4jz.js";import{i as d,t as f}from"./Stack-Dp2Fukd5.js";import{t as p}from"./Section-Caga5yen.js";import{t as m}from"./Section-Dpkyd44J.js";import{n as h,t as g}from"./Text--MQ9XwZh.js";import{n as _,t as v}from"./TextInput-DQ47lBVZ.js";import{t as y}from"./CheckboxInput-DV3bPoqw.js";import{en as b,tn as x}from"./iframe-C5jMfxr8.js";import{An as S,t as C}from"./src-rKsF_JcP.js";var w,T,E,D,O,k,A;e((()=>{w=t(n()),C(),o(),c(),h(),m(),f(),g(),v(),b(),y(),T=r(),E={title:`Lab/BottomSheet`,component:S,tags:[`autodocs`],parameters:{layout:`fullscreen`},decorators:[e=>(0,T.jsx)(`div`,{style:{minHeight:480,padding:32},children:(0,T.jsx)(e,{})})]},D={render:()=>{let[e,t]=(0,w.useState)(!1);return(0,T.jsxs)(T.Fragment,{children:[(0,T.jsx)(a,{label:`Open sheet`,onClick:()=>t(!0)}),(0,T.jsx)(S,{isOpen:e,onOpenChange:t,label:`Filters`,children:(0,T.jsx)(p,{padding:4,children:(0,T.jsxs)(d,{gap:4,children:[(0,T.jsx)(u,{level:3,children:`Filters`}),(0,T.jsx)(s,{}),(0,T.jsxs)(d,{gap:2,children:[(0,T.jsx)(l,{label:`In stock`,value:!1}),(0,T.jsx)(l,{label:`On sale`,value:!1}),(0,T.jsx)(l,{label:`Free shipping`,value:!1})]}),(0,T.jsx)(a,{label:`Apply`,onClick:()=>t(!1)})]})})})]})}},O={render:()=>{let[e,t]=(0,w.useState)(!1);return(0,T.jsxs)(T.Fragment,{children:[(0,T.jsx)(a,{label:`Open nearby places`,onClick:()=>t(!0)}),(0,T.jsx)(S,{isOpen:e,onOpenChange:t,label:`Nearby places`,height:`tall`,children:(0,T.jsx)(p,{padding:4,children:(0,T.jsxs)(d,{gap:3,children:[(0,T.jsx)(i,{type:`supporting`,color:`secondary`,children:`Drag the handle to resize between snap points; flick down to dismiss or up to expand. Escape also dismisses.`}),(0,T.jsx)(s,{}),Array.from({length:12},(e,t)=>(0,T.jsxs)(d,{gap:1,children:[(0,T.jsxs)(i,{type:`label`,children:[`Place `,t+1]}),(0,T.jsxs)(i,{type:`supporting`,color:`secondary`,children:[(.2+t*.3).toFixed(1),` mi away`]})]},t))]})})})]})}},k={render:()=>{let[e,t]=(0,w.useState)(!1);return(0,T.jsxs)(T.Fragment,{children:[(0,T.jsx)(a,{label:`Add a comment`,onClick:()=>t(!0)}),(0,T.jsx)(S,{isOpen:e,onOpenChange:t,label:`Add a comment`,height:`hug`,children:(0,T.jsx)(p,{padding:4,children:(0,T.jsxs)(d,{gap:4,children:[(0,T.jsx)(u,{level:3,children:`Add a comment`}),(0,T.jsx)(i,{type:`supporting`,color:`secondary`,children:`The sheet fits its content, up to 92% of the viewport.`}),(0,T.jsx)(s,{}),(0,T.jsx)(_,{label:`Title`,value:``}),(0,T.jsx)(x,{label:`Comment`,rows:4,value:``}),(0,T.jsx)(a,{label:`Post`,onClick:()=>t(!1)})]})})})]})}},D.parameters={...D.parameters,docs:{...D.parameters?.docs,source:{originalSource:`{
  render: () => {
    const [isOpen, setIsOpen] = useState(false);
    return <>
        <Button label="Open sheet" onClick={() => setIsOpen(true)} />
        <BottomSheet isOpen={isOpen} onOpenChange={setIsOpen} label="Filters">
          <Section padding={4}>
            <VStack gap={4}>
              <Heading level={3}>Filters</Heading>
              <Divider />
              <VStack gap={2}>
                <CheckboxInput label="In stock" value={false} />
                <CheckboxInput label="On sale" value={false} />
                <CheckboxInput label="Free shipping" value={false} />
              </VStack>
              <Button label="Apply" onClick={() => setIsOpen(false)} />
            </VStack>
          </Section>
        </BottomSheet>
      </>;
  }
}`,...D.parameters?.docs?.source}}},O.parameters={...O.parameters,docs:{...O.parameters?.docs,source:{originalSource:`{
  render: () => {
    const [isOpen, setIsOpen] = useState(false);
    return <>
        <Button label="Open nearby places" onClick={() => setIsOpen(true)} />
        <BottomSheet isOpen={isOpen} onOpenChange={setIsOpen} label="Nearby places" height="tall">
          <Section padding={4}>
            <VStack gap={3}>
              <Text type="supporting" color="secondary">
                Drag the handle to resize between snap points; flick down to
                dismiss or up to expand. Escape also dismisses.
              </Text>
              <Divider />
              {Array.from({
              length: 12
            }, (_, i) => <VStack key={i} gap={1}>
                  <Text type="label">Place {i + 1}</Text>
                  <Text type="supporting" color="secondary">
                    {(0.2 + i * 0.3).toFixed(1)} mi away
                  </Text>
                </VStack>)}
            </VStack>
          </Section>
        </BottomSheet>
      </>;
  }
}`,...O.parameters?.docs?.source}}},k.parameters={...k.parameters,docs:{...k.parameters?.docs,source:{originalSource:`{
  render: () => {
    const [isOpen, setIsOpen] = useState(false);
    return <>
        <Button label="Add a comment" onClick={() => setIsOpen(true)} />
        <BottomSheet isOpen={isOpen} onOpenChange={setIsOpen} label="Add a comment" height="hug">
          <Section padding={4}>
            <VStack gap={4}>
              <Heading level={3}>Add a comment</Heading>
              <Text type="supporting" color="secondary">
                The sheet fits its content, up to 92% of the viewport.
              </Text>
              <Divider />
              <TextInput label="Title" value="" />
              <TextArea label="Comment" rows={4} value="" />
              <Button label="Post" onClick={() => setIsOpen(false)} />
            </VStack>
          </Section>
        </BottomSheet>
      </>;
  }
}`,...k.parameters?.docs?.source}}},A=[`Showcase`,`TallSheet`,`HugHeight`]}))();export{k as HugHeight,D as Showcase,O as TallSheet,A as __namedExportsOrder,E as default};