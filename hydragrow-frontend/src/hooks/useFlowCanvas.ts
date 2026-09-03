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
 * Layout 1 React Flow node cho mỗi automation đã lưu ("Flow"). Không có edge —
 * mỗi Flow độc lập, click vào node chỉ mở chi tiết Blockly của chính nó (xem
 * `FlowDetailDrawer`), không nối sang Flow khác.
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

  const selectedScript = useMemo(
    () => (scripts ?? []).find((s) => s.id === selectedFlowId) ?? null,
    [scripts, selectedFlowId],
  );

  return {
    nodes,
    edges: [] as Edge[],
    selectedFlowId,
    selectedScript,
    isDrawerOpen: selectedFlowId !== null,
    isCreatingNew: selectedFlowId === 'new',
    openFlow: (id: string) => setSelectedFlowId(id),
    openNewFlow: () => setSelectedFlowId('new'),
    closeFlow: () => setSelectedFlowId(null),
  };
}
