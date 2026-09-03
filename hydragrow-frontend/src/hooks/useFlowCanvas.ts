import { useMemo, useState } from 'react';
import type { Node, Edge } from '@xyflow/react';
import type { UserScript } from '../types/automation';

const COLUMNS = 4;
const CELL_WIDTH = 220;
const CELL_HEIGHT = 140;

export interface FlowNodeData extends Record<string, unknown> {
  script: UserScript;
}

/**
 * Layout 1 React Flow node cho mỗi automation đã lưu ("Flow"). Mũi tên chain
 * được tính từ `next_flow_ids` trên canvas tổng quan.
 */
export function useFlowCanvas(scripts: UserScript[] | undefined) {
  const [selectedFlowId, setSelectedFlowId] = useState<string | 'new' | null>(null);

  const nodes: Node<FlowNodeData>[] = useMemo(
    () =>
      (scripts ?? []).map((script, i) => ({
        id: script.id,
        type: 'flowSummary',
        position: { x: (i % COLUMNS) * CELL_WIDTH, y: Math.floor(i / COLUMNS) * CELL_HEIGHT },
        data: { script },
      })),
    [scripts],
  );

  const edges: Edge[] = useMemo(() => {
    const idSet = new Set((scripts ?? []).map((s) => s.id));
    return (scripts ?? []).flatMap((s) =>
      (s.ir_json?.next_flow_ids ?? [])
        .filter((targetId) => idSet.has(targetId))
        .map((targetId) => ({
          id: `chain-${s.id}-${targetId}`,
          source: s.id,
          target: targetId,
          animated: true,
          style: { stroke: '#059669', strokeDasharray: '4 3' },
          label: 'kích hoạt tiếp',
          labelStyle: { fontSize: 10, fill: '#059669' },
        })),
    );
  }, [scripts]);

  const selectedScript = useMemo(
    () => (scripts ?? []).find((s) => s.id === selectedFlowId) ?? null,
    [scripts, selectedFlowId],
  );

  return {
    nodes,
    edges,
    selectedFlowId,
    selectedScript,
    isDrawerOpen: selectedFlowId !== null,
    isCreatingNew: selectedFlowId === 'new',
    openFlow: (id: string) => setSelectedFlowId(id),
    openNewFlow: () => setSelectedFlowId('new'),
    closeFlow: () => setSelectedFlowId(null),
  };
}
