import{i as e,s as t}from"./preload-helper-CT_b8DTk.js";import{t as n}from"./react-B7Te67-h.js";import{t as r}from"./jsx-runtime-DqZldVDK.js";import{pr as i,ur as a}from"./iframe-BkrJRCS_.js";var o,s,c,l,u,d,f,p;e((()=>{o=t(n()),a(),s=r(),c={title:`Core/RadioControl`,component:i,tags:[`autodocs`],argTypes:{label:{control:`text`,description:`Accessible name (aria-label) for a standalone control`},value:{control:`text`,description:`Value reported when this radio is selected`},name:{control:`text`,description:`HTML name shared by the radio group`},checked:{control:`boolean`,description:`Whether the radio is selected`},size:{control:`select`,options:[`sm`,`md`],description:`Size of the radio control`},isDisabled:{control:`boolean`,description:`Whether the radio is disabled`},isRequired:{control:`boolean`,description:`Whether the radio is required`}}},l={render:e=>{let[t,n]=(0,o.useState)(e.checked??!1);return(0,s.jsx)(i,{...e,checked:t,onChange:()=>{},onClick:()=>n(e=>!e)})},args:{label:`Email`,name:`notify`,value:`email`,checked:!0}},u={render:()=>(0,s.jsxs)(`div`,{style:{display:`flex`,alignItems:`center`,gap:24},children:[(0,s.jsx)(i,{label:`Small`,name:`sizes`,value:`sm`,size:`sm`,checked:!0,onChange:()=>{}}),(0,s.jsx)(i,{label:`Medium`,name:`sizes`,value:`md`,size:`md`,checked:!0,onChange:()=>{}})]})},d={render:()=>(0,s.jsxs)(`div`,{style:{display:`flex`,alignItems:`center`,gap:24},children:[(0,s.jsx)(i,{label:`Disabled unchecked`,name:`disabled`,value:`a`,checked:!1,isDisabled:!0,onChange:()=>{}}),(0,s.jsx)(i,{label:`Disabled checked`,name:`disabled`,value:`b`,checked:!0,isDisabled:!0,onChange:()=>{}})]})},f={render:()=>{let[e,t]=(0,o.useState)(`email`);return(0,s.jsx)(`div`,{style:{display:`flex`,flexDirection:`column`,gap:12},children:[{label:`Email`,value:`email`},{label:`SMS`,value:`sms`},{label:`Push`,value:`push`}].map(n=>(0,s.jsxs)(`label`,{style:{display:`flex`,alignItems:`center`,gap:8},children:[(0,s.jsx)(i,{name:`channel`,value:n.value,checked:e===n.value,onChange:t}),n.label]},n.value))})}},l.parameters={...l.parameters,docs:{...l.parameters?.docs,source:{originalSource:`{
  // A single radio can't normally be un-selected by clicking it (native radio
  // behavior — you deselect by choosing another in the group; see
  // ControlledGroup). To make this standalone demo interactive, drive the
  // checked state purely from onClick (which fires on every click), with a
  // no-op onChange to satisfy the controlled input. Mixing onChange (fires
  // only when the radio becomes checked) with onClick desyncs the toggle, so
  // onClick is the single source of truth here.
  render: args => {
    const [checked, setChecked] = useState(args.checked ?? false);
    return <RadioControl {...args} checked={checked} onChange={() => {}} onClick={() => setChecked(c => !c)} />;
  },
  args: {
    label: 'Email',
    name: 'notify',
    value: 'email',
    checked: true
  }
}`,...l.parameters?.docs?.source}}},u.parameters={...u.parameters,docs:{...u.parameters?.docs,source:{originalSource:`{
  render: () => <div style={{
    display: 'flex',
    alignItems: 'center',
    gap: 24
  }}>
      <RadioControl label="Small" name="sizes" value="sm" size="sm" checked onChange={() => {}} />
      <RadioControl label="Medium" name="sizes" value="md" size="md" checked onChange={() => {}} />
    </div>
}`,...u.parameters?.docs?.source}}},d.parameters={...d.parameters,docs:{...d.parameters?.docs,source:{originalSource:`{
  render: () => <div style={{
    display: 'flex',
    alignItems: 'center',
    gap: 24
  }}>
      <RadioControl label="Disabled unchecked" name="disabled" value="a" checked={false} isDisabled onChange={() => {}} />
      <RadioControl label="Disabled checked" name="disabled" value="b" checked isDisabled onChange={() => {}} />
    </div>
}`,...d.parameters?.docs?.source}}},f.parameters={...f.parameters,docs:{...f.parameters?.docs,source:{originalSource:`{
  render: () => {
    const [value, setValue] = useState('email');
    const options = [{
      label: 'Email',
      value: 'email'
    }, {
      label: 'SMS',
      value: 'sms'
    }, {
      label: 'Push',
      value: 'push'
    }];
    return <div style={{
      display: 'flex',
      flexDirection: 'column',
      gap: 12
    }}>
        {options.map(opt => <label key={opt.value} style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8
      }}>
            <RadioControl name="channel" value={opt.value} checked={value === opt.value} onChange={setValue} />
            {opt.label}
          </label>)}
      </div>;
  }
}`,...f.parameters?.docs?.source}}},p=[`Default`,`Sizes`,`Disabled`,`ControlledGroup`]}))();export{f as ControlledGroup,l as Default,d as Disabled,u as Sizes,p as __namedExportsOrder,c as default};