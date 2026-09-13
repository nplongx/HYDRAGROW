import { describe, expect, it } from 'vitest';
import { CAPABILITIES, ROLE_DEFAULT_SCOPES, type UserRole } from './Roles';

const CAPABILITY_SCOPES: Record<string, string[]> = {
  device: ['device:ota', 'device:network'],
  permissions: ['user:invite'],
  telemetry: ['read:telemetry'],
  control: ['control:pump', 'control:emergency'],
  recipes: ['write:config', 'recipe:write', 'script:write'],
};

function roleCanUseScopes(role: UserRole, scopes: string[]): boolean {
  const roleScopes = ROLE_DEFAULT_SCOPES[role];
  return roleScopes.includes('*') || scopes.every((scope) => roleScopes.includes(scope));
}

describe('Roles capability matrix', () => {
  it('matches CAPABILITIES access flags to ROLE_DEFAULT_SCOPES', () => {
    const capabilitiesById = new Map(CAPABILITIES.map((capability) => [capability.id, capability]));

    expect(new Set(capabilitiesById.keys())).toEqual(new Set(Object.keys(CAPABILITY_SCOPES)));

    for (const [id, scopes] of Object.entries(CAPABILITY_SCOPES)) {
      const capability = capabilitiesById.get(id);

      expect(capability, `missing capability row: ${id}`).toBeDefined();
      expect({
        admin: capability?.admin ?? false,
        operator: capability?.operator ?? false,
        viewer: capability?.viewer ?? false,
      }).toEqual({
        admin: roleCanUseScopes('admin', scopes),
        operator: roleCanUseScopes('operator', scopes),
        viewer: roleCanUseScopes('viewer', scopes),
      });
    }
  });
});
