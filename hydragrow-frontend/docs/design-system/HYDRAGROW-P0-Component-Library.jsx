<Frame name="HYDRAGROW P0 Component Gallery" flex="col" h={1320} w={1440} gap={16} p={24} bg="#F7F7F7">
  <Frame name="Gallery Header" flex="row" h={88} w="fill" justify="between" items="center" p={20} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
    <Frame name="Frame" flex="col" gap={4}>
      <Text name="Text" size={11} weight="bold" color="#6B6B6B">HYDRAGROW · P0 COMPONENT LIBRARY</Text>
      <Text name="Text" size={24} weight="bold" color="#2E7D32">Operational UI primitives</Text>
      <Text name="Text" size={12} color="#6B6B6B">1440px desktop · 8pt spacing · semantic states · touch target ≥ 44px</Text>
    </Frame>
    <Frame name="Frame" flex="row" gap={8} items="center" py={8} px={12} bg="#E8F5E9" rounded={999}>
      <Ellipse name="Ellipse" w={8} h={8} bg="#2E7D32" />
      <Text name="Text" size={11} weight={600} color="#2E7D32">P0 VISUAL SPEC</Text>
    </Frame>
  </Frame>
  <Frame name="Foundation Strip" flex="row" h={96} w="fill" gap={16} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
    <Frame name="Frame" flex="col" w={280} gap={6}>
      <Text name="Text" size={10} weight="bold" color="#9A9A9A">SURFACE + TYPE</Text>
      <Text name="Text" size={16} weight="bold" color="#111111">Calm operational hierarchy</Text>
      <Text name="Text" size={11} color="#6B6B6B">Inter · regular / medium / semibold / bold</Text>
    </Frame>
    <Frame name="Frame" flex="row" w={360} gap={8} items="center">
      <Frame name="Frame" flex="col" h={48} w={72} justify="center" items="center" bg="#FFFFFF" stroke="#EEEEEE" rounded={12}>
        <Text name="Text" size={9} color="#6B6B6B">surface</Text>
      </Frame>
      <Frame name="Frame" flex="col" h={48} w={72} justify="center" items="center" bg="#F7F7F7" stroke="#EEEEEE" rounded={12}>
        <Text name="Text" size={9} color="#6B6B6B">subtle</Text>
      </Frame>
      <Frame name="Frame" flex="col" h={48} w={72} justify="center" items="center" bg="#E8F5E9" rounded={12}>
        <Text name="Text" size={9} color="#2E7D32">normal</Text>
      </Frame>
      <Frame name="Frame" flex="col" h={48} w={72} justify="center" items="center" bg="#FFF3E0" rounded={12}>
        <Text name="Text" size={9} color="#B8590A">warning</Text>
      </Frame>
      <Frame name="Frame" flex="col" h={48} w={72} justify="center" items="center" bg="#FDECEA" rounded={12}>
        <Text name="Text" size={9} color="#C62828">fault</Text>
      </Frame>
    </Frame>
    <Frame name="Frame" flex="col" gap={6}>
      <Text name="Text" size={10} weight="bold" color="#9A9A9A">GEOMETRY</Text>
      <Text name="Text" size={12} weight={600} color="#111111">16px panel radius · 12/16px gaps · 40px control · 44px touch</Text>
      <Text name="Text" size={11} color="#6B6B6B">Critical actions stay visually unique.</Text>
    </Frame>
  </Frame>
  <Frame name="P0 Components Row 1" flex="row" w="fill" gap={16}>
    <Component name="AppShell" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">AppShell</Text>
      <Text name="Text" size={10} color="#6B6B6B">Navigation rail + utility + content canvas</Text>
      <Frame name="Frame" flex="row" h={126} w="fill" bg="#F7F7F7" rounded={12} overflow="hidden">
        <Frame name="Frame" flex="col" w={72} h="fill" gap={8} p={10} bg="#2E7D32">
          <Text name="Text" size={10} weight="bold" color="#FFFFFF">HG</Text>
          <Text name="Text" size={8} color="#E8F5E9">Overview</Text>
          <Text name="Text" size={8} color="#E8F5E9">Operations</Text>
          <Text name="Text" size={8} color="#E8F5E9">Journal</Text>
        </Frame>
        <Frame name="Frame" flex="col" grow={1} gap={8} p={12}>
          <Text name="Text" weight="bold" color="#2E7D32">Dashboard</Text>
          <Frame name="Frame" w={208} h={54} bg="#FFFFFF" stroke="#EEEEEE" rounded={10} />
        </Frame>
      </Frame>
    </Component>
    <Component name="PageHeader" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">PageHeader</Text>
      <Text name="Text" size={10} color="#6B6B6B">Eyebrow · title · description · actions</Text>
      <Frame name="Frame" flex="row" w="fill" justify="between" items="end" p={12} bg="#F7F7F7" rounded={12}>
        <Frame name="Frame" flex="col" gap={4}>
          <Text name="Text" size={9} weight="bold" color="#6B6B6B">STATION 01</Text>
          <Text name="Text" size={20} weight="bold" color="#2E7D32">Overview</Text>
          <Text name="Text" size={10} color="#6B6B6B">Live station health and telemetry</Text>
        </Frame>
        <Frame name="Frame" flex="col" py={9} px={12} bg="#2E7D32" rounded={10}>
          <Text name="Text" size={10} weight="bold" color="#FFFFFF">Refresh</Text>
        </Frame>
      </Frame>
    </Component>
    <Component name="Panel" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">Panel</Text>
      <Text name="Text" size={10} color="#6B6B6B">Default surface · subtle border · low elevation</Text>
      <Frame name="Frame" flex="col" w="fill" gap={10} p={12} bg="#FFFFFF" stroke="#EEEEEE" rounded={12} shadow="0 1 3 #00000014">
        <Text name="Text" size={12} weight="bold" color="#111111">Station health</Text>
        <Frame name="Frame" w={280} h={52} bg="#F7F7F7" rounded={10} />
      </Frame>
    </Component>
    <Component name="Button" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">Button</Text>
      <Text name="Text" size={10} color="#6B6B6B">Primary · secondary · danger · pending</Text>
      <Frame name="Frame" flex="row" gap={8} wrap>
        <Frame name="Frame" flex="col" py={10} px={12} bg="#2E7D32" rounded={10}>
          <Text name="Text" size={10} weight="bold" color="#FFFFFF">Primary</Text>
        </Frame>
        <Frame name="Frame" flex="col" py={9} px={12} bg="#FFFFFF" stroke="#2E7D32" rounded={10}>
          <Text name="Text" size={10} weight="bold" color="#2E7D32">Secondary</Text>
        </Frame>
        <Frame name="Frame" flex="col" py={10} px={12} bg="#C62828" rounded={10}>
          <Text name="Text" size={10} weight="bold" color="#FFFFFF">Danger</Text>
        </Frame>
        <Frame name="Frame" flex="col" py={10} px={12} bg="#FFF3E0" rounded={10}>
          <Text name="Text" size={10} weight="bold" color="#B8590A">Pending…</Text>
        </Frame>
      </Frame>
      <Text name="Text" size={9} color="#9A9A9A">One primary action per local group.</Text>
    </Component>
  </Frame>
  <Frame name="P0 Components Row 2" flex="row" w="fill" gap={16}>
    <Component name="StatusPill" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">StatusPill / DeviceStatePill</Text>
      <Text name="Text" size={10} color="#6B6B6B">Text + dot/icon + semantic treatment</Text>
      <Frame name="Frame" flex="row" gap={8} wrap>
        <Frame name="Frame" flex="row" gap={6} items="center" py={6} px={10} bg="#E8F5E9" rounded={999}>
          <Ellipse name="Ellipse" w={7} h={7} bg="#2E7D32" />
          <Text name="Text" size={9} weight="bold" color="#2E7D32">Online</Text>
        </Frame>
        <Frame name="Frame" flex="row" gap={6} items="center" py={6} px={10} bg="#FFF3E0" rounded={999}>
          <Ellipse name="Ellipse" w={7} h={7} bg="#B8590A" />
          <Text name="Text" size={9} weight="bold" color="#B8590A">Warning</Text>
        </Frame>
        <Frame name="Frame" flex="row" gap={6} items="center" py={6} px={10} bg="#F1F3F4" rounded={999}>
          <Ellipse name="Ellipse" w={7} h={7} bg="#5F6368" />
          <Text name="Text" size={9} weight="bold" color="#5F6368">Unavailable</Text>
        </Frame>
      </Frame>
      <Text name="Text" size={9} color="#9A9A9A">Never color-only semantics.</Text>
    </Component>
    <Component name="TelemetryCard" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">TelemetryCard</Text>
      <Text name="Text" size={10} color="#6B6B6B">Value &gt; unit &gt; label &gt; freshness</Text>
      <Frame name="Frame" flex="col" w="fill" gap={4} p={12} bg="#F7F7F7" rounded={12}>
        <Text name="Text" size={10} weight={600} color="#6B6B6B">pH</Text>
        <Frame name="Frame" flex="row" gap={5} items="end">
          <Text name="Text" size={24} weight="bold" color="#111111">6.2</Text>
          <Text name="Text" size={10} color="#6B6B6B">pH</Text>
        </Frame>
        <Text name="Text" size={9} color="#2E7D32">Updated 8s ago · stable</Text>
      </Frame>
      <Frame name="Frame" flex="col" w="fill" py={7} px={10} bg="#FDECEA" rounded={10}>
        <Text name="Text" size={9} weight={600} color="#C62828">STALE · value not live</Text>
      </Frame>
    </Component>
    <Component name="TelemetryGroup" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">TelemetryGroup</Text>
      <Text name="Text" size={10} color="#6B6B6B">Group by operational meaning</Text>
      <Frame name="Frame" flex="row" w="fill" gap={8}>
        <Frame name="Frame" flex="col" grow={1} gap={3} p={10} bg="#F7F7F7" rounded={10}>
          <Text name="Text" size={9} color="#6B6B6B">EC</Text>
          <Text name="Text" size={17} weight="bold" color="#111111">1.8</Text>
          <Text name="Text" size={8} color="#6B6B6B">mS/cm</Text>
        </Frame>
        <Frame name="Frame" flex="col" grow={1} gap={3} p={10} bg="#F7F7F7" rounded={10}>
          <Text name="Text" size={9} color="#6B6B6B">pH</Text>
          <Text name="Text" size={17} weight="bold" color="#111111">6.2</Text>
          <Text name="Text" size={8} color="#6B6B6B">normal</Text>
        </Frame>
        <Frame name="Frame" flex="col" grow={1} gap={3} p={10} bg="#F7F7F7" rounded={10}>
          <Text name="Text" size={9} color="#6B6B6B">Temp</Text>
          <Text name="Text" size={17} weight="bold" color="#111111">23.4</Text>
          <Text name="Text" size={8} color="#6B6B6B">°C</Text>
        </Frame>
      </Frame>
    </Component>
    <Component name="ActuatorCard" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">ActuatorCard</Text>
      <Text name="Text" size={10} color="#6B6B6B">Physical state dominates pending command</Text>
      <Frame name="Frame" flex="row" w="fill" justify="between" items="center" p={12} bg="#F7F7F7" rounded={12}>
        <Frame name="Frame" flex="col" gap={4}>
          <Text name="Text" size={11} weight="bold" color="#111111">Pump A</Text>
          <Frame name="Frame" flex="col" py={5} px={8} bg="#E8F5E9" rounded={999}>
            <Text name="Text" size={8} weight="bold" color="#2E7D32">RUNNING</Text>
          </Frame>
          <Text name="Text" size={8} color="#6B6B6B">Physical feedback confirmed</Text>
        </Frame>
        <Frame name="Frame" flex="col" h={24} w={44} p={3} bg="#2E7D32" rounded={999}>
          <Ellipse name="Ellipse" w={18} h={18} bg="#FFFFFF" />
        </Frame>
      </Frame>
      <Frame name="Frame" flex="col" w="fill" py={7} px={10} bg="#FFF3E0" rounded={10}>
        <Text name="Text" size={9} weight={600} color="#B8590A">Command pending · not physical success</Text>
      </Frame>
    </Component>
  </Frame>
  <Frame name="P0 Components Row 3" flex="row" w="fill" gap={16}>
    <Component name="QuickActionBar" flex="col" h={208} w={336} gap={10} p={16} bg="#FFFFFF" stroke="#EEEEEE" rounded={16}>
      <Text name="Text" size={13} weight="bold" color="#111111">QuickActionBar</Text>
      <Text name="Text" size={10} color="#6B6B6B">High-