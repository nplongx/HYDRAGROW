<Frame name="P2 Automation Internal Form Grammar" w={1440} h={1720} bg="#F7F7F7" p={24} flex="col" gap={16}>
  <Frame name="P2 Header" w="fill" h={128} bg="#FFFFFF" rounded={16} p={20} flex="row" justify="between" items="center" stroke="#EEEEEE">
    <Frame w={900} h={80} flex="col" gap={4}><Text size={11} weight={700} color="#6B6B6B">HYDRAGROW · P2 AUTOMATION</Text><Text size={24} weight={700} color="#14532D">Internal form grammar</Text><Text size={12} color="#6B6B6B">Page-local surfaces · inherits InputGroup, Button, Switch, Badge, Panel/SubCard</Text></Frame>
    <Frame w={120} h={32} bg="#F3F4F6" rounded={999} flex="row" justify="center" items="center"><Text size={9} weight={700} color="#374151">P2 · PAGE LOCAL</Text></Frame>
  </Frame>
  <Frame name="P2 Row 1" w="fill" h={250} flex="row" gap={16}>
    <Frame name="FieldGroup" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Frame w="fill" h={34} flex="row" items="center" justify="between"><Text size={13} weight={700} color="#111111">FieldGroup</Text><Text size={8} weight={700} color="#047857">DEFAULT + INVALID</Text></Frame>
      <Frame w="fill" h={68} flex="col" gap={5}><Text size={10} weight={700} color="#111111">Dose volume</Text><Frame w="fill" h={36} bg="#FFFFFF" rounded={10} p={9} stroke="#D1D5DB"><Text size={10} color="#111111">25</Text></Frame><Text size={8} color="#6B6B6B">mL per cycle · safe range 5–100 mL</Text></Frame>
      <Frame w="fill" h={68} flex="col" gap={5}><Text size={10} weight={700} color="#111111">Target EC</Text><Frame w="fill" h={36} bg="#FFFFFF" rounded={10} p={9} stroke="#FCA5A5"><Text size={10} color="#111111">2.8</Text></Frame><Text size={8} color="#991B1B">Invalid: target exceeds recipe limit</Text></Frame>
      <Text size={8} color="#6B6B6B">Validation message stays beside the affected field.</Text>
    </Frame>
    <Frame name="InputWithSuffix" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Frame w="fill" h={34} flex="col" gap={3}><Text size={13} weight={700} color="#111111">InputWithSuffix</Text><Text size={8} color="#6B6B6B">Numeric value + fixed unit</Text></Frame>
      <Frame w="fill" h={44} bg="#FFFFFF" rounded={10} flex="row" items="center" stroke="#9CA3AF"><Text size={12} color="#111111">30</Text><Frame w="fill" h={1}/><Text size={9} weight={700} color="#6B6B6B">seconds</Text></Frame>
      <Frame w="fill" h={44} bg="#FFFFFF" rounded={10} flex="row" items="center" stroke="#D1D5DB"><Text size={12} color="#111111">2.0</Text><Frame w="fill" h={1}/><Text size={9} weight={700} color="#6B6B6B">mS/cm</Text></Frame>
      <Text size={8} color="#6B6B6B">Unit remains visible on compact/mobile layouts.</Text>
    </Frame>
  </Frame>
  <Frame name="P2 Row 2" w="fill" h={250} flex="row" gap={16}>
    <Frame name="InputWithButton" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Frame w="fill" h={34} flex="col" gap={3}><Text size={13} weight={700} color="#111111">InputWithButton</Text><Text size={8} color="#6B6B6B">Attached field action with explicit label</Text></Frame>
      <Frame w="fill" h={44} bg="#FFFFFF" rounded={10} flex="row" items="center" stroke="#9CA3AF"><Text size={10} color="#111111">station.alpha</Text><Frame w="fill" h={1}/><Frame w={70} h={32} bg="#15803D" rounded={9} flex="row" justify="center" items="center"><Text size={9} weight={700} color="#FFFFFF">Load</Text></Frame></Frame>
      <Frame w="fill" h={44} flex="row" gap={8}><Frame w={90} h={32} bg="#FFFFFF" rounded={10} flex="row" justify="center" items="center" stroke="#D1D5DB"><Text size={9} weight={700} color="#374151">Browse</Text></Frame><Frame w={90} h={32} bg="#F7F7F7" rounded={10} flex="row" justify="center" items="center"><Text size={9} weight={700} color="#9CA3AF">Disabled</Text></Frame></Frame>
      <Text size={8} color="#6B6B6B">Required recovery actions use text labels, not icon-only controls.</Text>
    </Frame>
    <Frame name="PillsSelector" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Frame w="fill" h={34} flex="col" gap={3}><Text size={13} weight={700} color="#111111">PillsSelector</Text><Text size={8} color="#6B6B6B">Single choice with clear active treatment</Text></Frame>
      <Frame w="fill" h={44} flex="row" gap={8}><Frame w={120} h={36} bg="#15803D" rounded={999} flex="row" justify="center" items="center"><Text size={9} weight={700} color="#FFFFFF">Morning</Text></Frame><Frame w={120} h={36} bg="#FFFFFF" rounded={999} flex="row" justify="center" items="center" stroke="#D1D5DB"><Text size={9} weight={700} color="#374151">Midday</Text></Frame><Frame w={120} h={36} bg="#FFFFFF" rounded={999} flex="row" justify="center" items="center" stroke="#D1D5DB"><Text size={9} weight={700} color="#374151">Evening</Text></Frame></Frame>
      <Frame w="fill" h={40} bg="#F0FDF4" rounded={10} flex="row" items="center" p={10}><Text size={9} color="#047857">Selected: Morning · 06:00–08:00</Text></Frame>
      <Text size={8} color="#6B6B6B">Touch target ≥44px for interactive controls; overflow beats ambiguous wrapping.</Text>
    </Frame>
  </Frame>
<Frame name="P2 Row 3" w="fill" h={250} flex="row" gap={16}>
  <Frame name="Segmented" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
    <Text size={13} weight={700} color="#111111">Segmented</Text>
    <Text size={8} color="#6B6B6B">Finite mode switch; focus and pending stay explicit</Text>
    <Frame w="fill" h={42} bg="#F3F4F6" rounded={10} p={3} flex="row" gap={3}><Frame w="fill" h={36} bg="#FFFFFF" rounded={8} flex="row" justify="center" items="center" stroke="#E5E7EB"><Text size={9} weight={700} color="#111111">Auto</Text></Frame><Frame w="fill" h={36} bg="#F3F4F6" rounded={8} flex="row" justify="center" items="center"><Text size={9} color="#6B7280">Manual</Text></Frame><Frame w="fill" h={36} bg="#F3F4F6" rounded={8} flex="row" justify="center" items="center"><Text size={9} color="#6B7280">Off</Text></Frame></Frame>
    <Frame w="fill" h={48} bg="#F0FDF4" rounded={10} p={10}><Text size={9} weight={700} color="#047857">Auto mode</Text><Text size={8} color="#047857">Flow controls actuator by rule.</Text></Frame>
    <Text size={8} color="#6B6B6B">Keyboard focus remains visible; labels stay readable.</Text>
  </Frame>
  <Frame name="ChipsRow" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
    <Text size={13} weight={700} color="#111111">ChipsRow + Chip</Text>
    <Text size={8} color="#6B6B6B">Compact multi-value summary</Text>
    <Frame w="fill" h={42} flex="row" gap={8}><Frame w={120} h={32} bg="#ECFDF5" rounded={999} flex="row" justify="center" items="center"><Text size={8} weight={700} color="#065F46">EC above 2.5</Text></Frame><Frame w={120} h={32} bg="#EFF6FF" rounded={999} flex="row" justify="center" items="center"><Text size={8} weight={700} color="#075985">pH below 5.8</Text></Frame><Frame w={120} h={32} bg="#F3F4F6" rounded={999} flex="row" justify="center" items="center"><Text size={8} weight={700} color="#4B5563">Stage Growth</Text></Frame></Frame>
    <Frame w="fill" h={40} bg="#FFFBEB" rounded={10} p={10}><Text size={9} color="#92400E">2 conditions; all must pass</Text></Frame>
    <Text size={8} color="#6B6B6B">Chips summarize; full editable meaning stays in the parent editor.</Text>
  </Frame>
</Frame>

<Frame name="P2 Rows 4 and 5" w="fill" flex="col" gap={16}>
  <Frame name="P2 Row 4" w="fill" h={250} flex="row" gap={16}>
    <Frame name="ToggleRow" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Text size={13} weight={700} color="#111111">ToggleRow</Text><Text size={8} color="#6B6B6B">Behavior label + switch + async state</Text>
      <Frame w="fill" h={48} bg="#F7F7F7" rounded={12} p={10} flex="row" items="center"><Frame w="fill" h={28} flex="col" gap={2}><Text size={9} weight={700} color="#111111">Enable automation</Text><Text size={7} color="#6B6B6B">Allow this flow to issue configured commands.</Text></Frame><Frame w={46} h={26} bg="#15803D" rounded={999} p={3}><Ellipse w={20} h={20} bg="#FFFFFF"/></Frame></Frame>
      <Frame w="fill" h={48} bg="#F7F7F7" rounded={12} p={10} flex="row" items="center"><Frame w="fill" h={28} flex="col" gap={2}><Text size={9} weight={700} color="#111111">Dry run only</Text><Text size={7} color="#6B6B6B">Simulation; never confirms physical state.</Text></Frame><Frame w={46} h={26} bg="#E5E7EB" rounded={999} p={3}><Ellipse w={20} h={20} bg="#FFFFFF"/></Frame></Frame>
      <Text size={8} color="#6B6B6B">Label describes behavior, not implementation.</Text>
    </Frame>
    <Frame name="ConfigCard" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Frame w="fill" h={30} flex="row" items="center" justify="between"><Text size={13} weight={700} color="#111111">ConfigCard</Text><Text size={8} weight={700} color="#4F46E5">CONFIG</Text></Frame>
      <Frame w="fill" h={86} bg="#F8FAFC" rounded={12} p={10} flex="col" gap={6}><Text size={9} weight={700} color="#111111">Station override</Text><Frame w="fill" h={40} flex="row" gap={8}><Frame w="fill" h={40} bg="#FFFFFF" rounded={9} p={8} stroke="#D1D5DB"><Text size={7} color="#6B7280">EC target</Text><Text size={9} weight={700} color="#111111">2.0</Text></Frame><Frame w="fill" h={40} bg="#FFFFFF" rounded={9} p={8} stroke="#D1D5DB"><Text size={7} color="#6B7280">Dose cap</Text><Text size={9} weight={700} color="#111111">60 mL</Text></Frame></Frame></Frame>
      <Text size={8} color="#3730A3">Override active; inherited recipe remains visible.</Text><Text size={8} color="#6B6B6B">Indigo is the existing config exception.</Text>
    </Frame>
  </Frame>
  <Frame name="P2 Row 5" w="fill" h={250} flex="row" gap={16}>
    <Frame name="InspectorShell" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Frame w="fill" h={30} flex="row" items="center" justify="between"><Text size={13} weight={700} color="#111111">InspectorShell</Text><Text size={8} weight={700} color="#4B5563">SELECTED NODE</Text></Frame>
      <Frame w="fill" h={98} bg="#F7F7F7" rounded={12} p={10} flex="row" gap={10}><Frame w={130} h={78} flex="col" gap={5}><Text size={8} color="#6B7280">Node type</Text><Text size={10} weight={700} color="#14532D">CONDITION</Text><Text size={7} color="#6B7280">Node 02; valid</Text></Frame><Frame w={1} h={78} bg="#E5E7EB"/><Frame w="fill" h={78} flex="col" gap={6}><Text size={8} weight={700} color="#111111">Condition</Text><Frame w="fill" h={40} bg="#FFFFFF" rounded={9} p={8} stroke="#D1D5DB"><Text size={8} color="#111111">EC above 2.5 then Dose</Text></Frame><Text size={7} color="#6B6B6B">Validation stays beside the affected field.</Text></Frame></Frame>
      <Text size={8} color="#6B6B6B">Mobile: stack node identity before fields; preserve validation and actions.</Text>
    </Frame>
    <Frame name="SafeNote" w={680} h={250} bg="#FFFFFF" rounded={16} p={16} flex="col" gap={8} stroke="#EEEEEE">
      <Text size={13} weight={700} color="#111111">SafeNote</Text><Text size={8} color="#6B6B6B">Safety and interlock note inside automation forms</Text>
      <Frame w="fill" h={98} bg="#FEF2F2" rounded={12} p={12} flex="col" gap={7}><Frame w="fill" h={22} flex="row" items="center" gap={8}><Ellipse w={20} h={20} bg="#DC2626"/><Text size={10} weight={700} color="#991B1B">Safety interlock</Text></Frame><Text size={8} color="#991B1B">Pump A cannot run below minimum tank level.</Text><Frame w={150} h={28} bg="#FFFFFF" rounded={9} flex="row" justify="center" items="center" stroke="#FECACA"><Text size={8} weight={700} color="#991B1B">View details</Text></Frame></Frame>
      <Text size={8} color="#6B6B6B">Critical meaning = icon + explicit text + semantic red.</Text>
    </Frame>
  </Frame>
  <Frame name="P2 Rules" w="fill" h={140} bg="#FFFFFF" rounded={16} p={16} flex="row" gap={24} stroke="#EEEEEE">
    <Frame w={430} h={100} flex="col" gap={6}><Text size={12} weight={700} color="#111111">Responsive</Text><Text size={8} color="#6B6B6B">Below 768px: cards stack, inspector identity precedes fields, chip rows scroll, labels stay readable.</Text></Frame>
    <Frame w={430} h={100} flex="col" gap={6}><Text size={12} weight={700} color="#111111">State grammar</Text><Text size={8} color="#6B6B6B">Default, focus, filled, disabled, invalid, warning, pending use text, icon, border or fill, and affordance together.</Text></Frame>
    <Frame w={430} h={100} flex="col" gap={6}><Text size={12} weight={700} color="#111111">Promotion rule</Text><Text size={8} color="#6B6B6B">Keep page-local. Promote only after independent reuse outside automation.</Text></Frame>
  </Frame>
</Frame>

</Frame>
