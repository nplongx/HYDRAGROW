import { describe, expect, it } from 'vitest';
import { adaptJournalEvent } from './journal';

describe('journal transport adapter', () => {
  it('normalizes wire enums and resource id to UI contract', () => {
    const event = adaptJournalEvent({
      id: '42', device_id: 'dev-1', level: 'ERROR', category: 'USER_ACTION',
      title: 'Control', message: 'Pump changed', timestamp: 1700000000,
    });
    expect(event.id).toBe(42);
    expect(event.level).toBe('error');
    expect(event.category).toBe('user_action');
  });

  it('fails safe to stable enum values for unknown wire strings', () => {
    const event = adaptJournalEvent({
      id: 1, device_id: 'dev-1', level: 'future', category: 'future',
      title: 'x', message: 'x', timestamp: 1,
    });
    expect(event.level).toBe('info');
    expect(event.category).toBe('system');
  });
});
