# HYDRAGROW Design Lab

## Purpose

Explore multiple UI compositions without spending Figma/Penpot MCP calls on every iteration.

```text
Design spec
  -> React concept A/B/C
  -> browser render
  -> screenshot / inspect
  -> visual comparison
  -> selected direction
  -> production React
  -> Penpot final artifact
```

## Local playground

Run frontend normally, then open:

```text
/design-lab?concept=a
/design-lab?concept=b
/design-lab?concept=c
```

The page uses fixture data. It is a visual composition playground, not a telemetry source.

### Concept contract

- Same HydraGrow design tokens.
- Same information model.
- Same semantic states and labels.
- Different composition, hierarchy, density, and spatial grouping.
- Real React DOM, responsive CSS, keyboard-accessible controls.
- No dependency on Penpot, Figma, or Recraft at runtime.

## Tool roles

| Tool | Role |
| --- | --- |
| React | Production implementation + visual playground |
| Browser | Render, screenshot, inspect, compare |
| Penpot | Design canvas, components, tokens, selected final layout |
| OpenDesign/OpenCode | Generate and iterate code concepts |
| Recraft | Custom artwork only; optional when credits exist |

Do not use Penpot MCP for every visual iteration. Use it when reading/writing the selected design artifact, design-system maintenance, or handoff needs justify the round trip.

## Penpot local instance

Current self-hosted instance:

```text
http://localhost:9001
```

The official Docker Compose stack is healthy. The bundled `penpot-mcp` service is internal to the Compose network; the frontend proxies MCP routes. In multi-user mode, MCP requires a Penpot `userToken`, so OpenDesign should not hard-code a token into repository config. Connect MCP only after a Penpot account/file is active and the local token is intentionally supplied.

The MCP routes exposed by the frontend are:

```text
http://localhost:9001/mcp/stream
http://localhost:9001/mcp/sse
http://localhost:9001/mcp/ws
```

## Promotion rule

Do not rewrite production components merely to make a concept look different. Keep composition-specific code in `src/pages/DesignLab.tsx` until a direction is selected. Then extract stable patterns into existing `src/components/ui` / `src/components/layout` contracts and replace the production page composition.
