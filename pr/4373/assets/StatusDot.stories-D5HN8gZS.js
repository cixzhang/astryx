import{i as e}from"./preload-helper-CT_b8DTk.js";import{t}from"./jsx-runtime-DqZldVDK.js";import{D as n,E as r}from"./iframe-CDLIismy.js";function i(){return(0,a.jsx)(`svg`,{viewBox:`0 0 8 8`,width:8,height:8,"aria-hidden":`true`,children:(0,a.jsx)(`rect`,{x:2.4,y:2.4,width:3.2,height:3.2,fill:`currentColor`,transform:`rotate(45 4 4)`})})}var a,o,s,c,l,u,d,f,p,m,h;e((()=>{r(),a=t(),o={title:`Core/StatusDot`,component:n,tags:[`autodocs`],argTypes:{variant:{control:`select`,options:[`success`,`warning`,`error`,`accent`,`neutral`],description:`Semantic variant pairing colour with a distinct built-in shape (success check, warning exclamation, error cross, neutral ring, accent filled)`},label:{control:`text`,description:`Accessible label`},isPulsing:{control:`boolean`,description:`Pulse animation`},tooltip:{control:`text`,description:`Tooltip text on hover`}}},s={args:{variant:`success`,label:`Online`}},c={render:()=>(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`8px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`success`,label:`Positive`}),(0,a.jsx)(n,{variant:`warning`,label:`Warning`}),(0,a.jsx)(n,{variant:`error`,label:`Negative`}),(0,a.jsx)(n,{variant:`accent`,label:`Info`}),(0,a.jsx)(n,{variant:`neutral`,label:`Neutral`})]})},l={render:()=>(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`8px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`success`,label:`Live`,isPulsing:!0}),(0,a.jsx)(n,{variant:`warning`,label:`Processing`,isPulsing:!0}),(0,a.jsx)(n,{variant:`error`,label:`Error`,isPulsing:!0})]})},u={render:()=>(0,a.jsxs)(`div`,{style:{display:`flex`,flexDirection:`column`,gap:`8px`},children:[(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`8px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`success`,label:`Online`}),(0,a.jsx)(`span`,{children:`Online`})]}),(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`8px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`warning`,label:`Away`}),(0,a.jsx)(`span`,{children:`Away`})]}),(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`8px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`error`,label:`Offline`}),(0,a.jsx)(`span`,{children:`Offline`})]}),(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`8px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`neutral`,label:`Unknown`}),(0,a.jsx)(`span`,{children:`Unknown`})]})]})},d={render:()=>(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`16px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`success`,label:`Online`,tooltip:`Online`}),(0,a.jsx)(n,{variant:`warning`,label:`Away`,tooltip:`Away`}),(0,a.jsx)(n,{variant:`error`,label:`Offline`,tooltip:`Offline`}),(0,a.jsx)(n,{variant:`neutral`,label:`Unknown`,tooltip:`Unknown`})]})},f=[{variant:`success`,label:`Success (check)`},{variant:`warning`,label:`Warning (exclamation)`},{variant:`error`,label:`Error (cross)`},{variant:`neutral`,label:`Neutral (ring)`},{variant:`accent`,label:`Accent (filled)`}],p=8,m={parameters:{docs:{description:{story:"All variants at native 8px (1x) for colour-blind verification, a magnified row to inspect glyph geometry, and the `icon` override."}}},render:()=>(0,a.jsxs)(`div`,{style:{display:`flex`,flexDirection:`column`,gap:`32px`},children:[(0,a.jsxs)(`section`,{children:[(0,a.jsx)(`h4`,{style:{margin:`0 0 12px`},children:`Actual size (1x, 8px)`}),(0,a.jsx)(`div`,{style:{display:`flex`,gap:`24px`,alignItems:`center`},children:f.map(({variant:e,label:t})=>(0,a.jsxs)(`div`,{style:{display:`flex`,flexDirection:`column`,alignItems:`center`,gap:`8px`},children:[(0,a.jsx)(n,{variant:e,label:t}),(0,a.jsx)(`span`,{style:{fontSize:`11px`},children:t})]},e))})]}),(0,a.jsxs)(`section`,{children:[(0,a.jsxs)(`h4`,{style:{margin:`0 0 12px`},children:[`Magnified `,p,`x (geometry)`]}),(0,a.jsx)(`div`,{style:{display:`flex`,gap:`24px`,alignItems:`center`},children:f.map(({variant:e,label:t})=>(0,a.jsx)(`div`,{style:{width:8*p,height:8*p,display:`flex`,alignItems:`center`,justifyContent:`center`,border:`1px solid rgba(128,128,128,0.4)`,borderRadius:`8px`},children:(0,a.jsx)(`div`,{style:{transform:`scale(${p})`},children:(0,a.jsx)(n,{variant:e,label:t})})},e))})]}),(0,a.jsxs)(`section`,{children:[(0,a.jsx)(`h4`,{style:{margin:`0 0 12px`},children:`Icon override`}),(0,a.jsxs)(`div`,{style:{display:`flex`,gap:`24px`,alignItems:`center`},children:[(0,a.jsx)(n,{variant:`success`,label:`Verified`,icon:(0,a.jsx)(i,{})}),(0,a.jsx)(n,{variant:`accent`,label:`Featured`,icon:(0,a.jsx)(i,{})}),(0,a.jsx)(`span`,{style:{fontSize:`11px`},children:`icon replaces the built-in glyph`})]})]})]})},s.parameters={...s.parameters,docs:{...s.parameters?.docs,source:{originalSource:`{
  args: {
    variant: 'success',
    label: 'Online'
  }
}`,...s.parameters?.docs?.source}}},c.parameters={...c.parameters,docs:{...c.parameters?.docs,source:{originalSource:`{
  render: () => <div style={{
    display: 'flex',
    gap: '8px',
    alignItems: 'center'
  }}>
      <StatusDot variant="success" label="Positive" />
      <StatusDot variant="warning" label="Warning" />
      <StatusDot variant="error" label="Negative" />
      <StatusDot variant="accent" label="Info" />
      <StatusDot variant="neutral" label="Neutral" />
    </div>
}`,...c.parameters?.docs?.source}}},l.parameters={...l.parameters,docs:{...l.parameters?.docs,source:{originalSource:`{
  render: () => <div style={{
    display: 'flex',
    gap: '8px',
    alignItems: 'center'
  }}>
      <StatusDot variant="success" label="Live" isPulsing />
      <StatusDot variant="warning" label="Processing" isPulsing />
      <StatusDot variant="error" label="Error" isPulsing />
    </div>
}`,...l.parameters?.docs?.source}}},u.parameters={...u.parameters,docs:{...u.parameters?.docs,source:{originalSource:`{
  render: () => <div style={{
    display: 'flex',
    flexDirection: 'column',
    gap: '8px'
  }}>
      <div style={{
      display: 'flex',
      gap: '8px',
      alignItems: 'center'
    }}>
        <StatusDot variant="success" label="Online" />
        <span>Online</span>
      </div>
      <div style={{
      display: 'flex',
      gap: '8px',
      alignItems: 'center'
    }}>
        <StatusDot variant="warning" label="Away" />
        <span>Away</span>
      </div>
      <div style={{
      display: 'flex',
      gap: '8px',
      alignItems: 'center'
    }}>
        <StatusDot variant="error" label="Offline" />
        <span>Offline</span>
      </div>
      <div style={{
      display: 'flex',
      gap: '8px',
      alignItems: 'center'
    }}>
        <StatusDot variant="neutral" label="Unknown" />
        <span>Unknown</span>
      </div>
    </div>
}`,...u.parameters?.docs?.source}}},d.parameters={...d.parameters,docs:{...d.parameters?.docs,source:{originalSource:`{
  render: () => <div style={{
    display: 'flex',
    gap: '16px',
    alignItems: 'center'
  }}>
      <StatusDot variant="success" label="Online" tooltip="Online" />
      <StatusDot variant="warning" label="Away" tooltip="Away" />
      <StatusDot variant="error" label="Offline" tooltip="Offline" />
      <StatusDot variant="neutral" label="Unknown" tooltip="Unknown" />
    </div>
}`,...d.parameters?.docs?.source}}},m.parameters={...m.parameters,docs:{...m.parameters?.docs,source:{originalSource:`{
  parameters: {
    docs: {
      description: {
        story: 'All variants at native 8px (1x) for colour-blind verification, a magnified row to inspect glyph geometry, and the \`icon\` override.'
      }
    }
  },
  render: () => <div style={{
    display: 'flex',
    flexDirection: 'column',
    gap: '32px'
  }}>
      <section>
        <h4 style={{
        margin: '0 0 12px'
      }}>Actual size (1x, 8px)</h4>
        <div style={{
        display: 'flex',
        gap: '24px',
        alignItems: 'center'
      }}>
          {LEGIBILITY_VARIANTS.map(({
          variant,
          label
        }) => <div key={variant} style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '8px'
        }}>
              <StatusDot variant={variant} label={label} />
              <span style={{
            fontSize: '11px'
          }}>{label}</span>
            </div>)}
        </div>
      </section>

      <section>
        <h4 style={{
        margin: '0 0 12px'
      }}>Magnified {MAGNIFY}x (geometry)</h4>
        <div style={{
        display: 'flex',
        gap: '24px',
        alignItems: 'center'
      }}>
          {LEGIBILITY_VARIANTS.map(({
          variant,
          label
        }) => <div key={variant} style={{
          width: 8 * MAGNIFY,
          height: 8 * MAGNIFY,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          border: '1px solid rgba(128,128,128,0.4)',
          borderRadius: '8px'
        }}>
              <div style={{
            transform: \`scale(\${MAGNIFY})\`
          }}>
                <StatusDot variant={variant} label={label} />
              </div>
            </div>)}
        </div>
      </section>

      <section>
        <h4 style={{
        margin: '0 0 12px'
      }}>Icon override</h4>
        <div style={{
        display: 'flex',
        gap: '24px',
        alignItems: 'center'
      }}>
          <StatusDot variant="success" label="Verified" icon={<DiamondIcon />} />
          <StatusDot variant="accent" label="Featured" icon={<DiamondIcon />} />
          <span style={{
          fontSize: '11px'
        }}>
            icon replaces the built-in glyph
          </span>
        </div>
      </section>
    </div>
}`,...m.parameters?.docs?.source},description:{story:`Glyph legibility reference. StatusDot is a fixed 8px dot, so 8px IS its
smallest (and only) size. The top row is the native 1x rendering — the
artifact to eyeball and to run a colour-blind sim against on the deployed
Storybook. The middle row magnifies each dot so the glyph geometry is
inspectable (check vs cross in particular). The bottom row shows the \`icon\`
prop overriding the built-in glyph.`,...m.parameters?.docs?.description}}},h=[`Default`,`Variants`,`Pulsing`,`StatusIndicators`,`WithTooltip`,`GlyphLegibility`]}))();export{s as Default,m as GlyphLegibility,l as Pulsing,u as StatusIndicators,c as Variants,d as WithTooltip,h as __namedExportsOrder,o as default};