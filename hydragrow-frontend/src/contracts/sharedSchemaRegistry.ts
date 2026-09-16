import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export type ContractField = {
    type: 'string' | 'number' | 'integer' | 'boolean' | 'object' | 'array';
    required: boolean;
    nullable: boolean;
    enum?: string[];
    format?: 'date-time';
    finite?: boolean;
    precision?: string;
    canonical?: string;
    compatibility_alias?: boolean;
};

export type SchemaContract = {
    version: number;
    layer: 'canonical_wire';
    fixture: string;
    required: string[];
    fields: Record<string, ContractField>;
};

export type SchemaRegistry = {
    artifact_kind: 'contract_registry_metadata';
    registry_version: string;
    schema_dialect: string;
    contracts: Record<string, SchemaContract>;
};

export function loadSharedSchemaRegistry(): SchemaRegistry {
    const path = resolve(process.cwd(), '../schema/contracts/registry.json');
    return JSON.parse(readFileSync(path, 'utf8')) as SchemaRegistry;
}

export function loadSharedSchemaFixture(contractName: string): unknown {
    const registry = loadSharedSchemaRegistry();
    const contract = registry.contracts[contractName];
    if (!contract) throw new Error(`Unknown schema contract: ${contractName}`);
    const path = resolve(process.cwd(), '../schema/contracts', contract.fixture);
    return JSON.parse(readFileSync(path, 'utf8')) as unknown;
}

function resolvePath(value: unknown, segments: string[]): unknown[] {
    if (segments.length === 0) return [value];
    const [head, ...tail] = segments;
    if (head === '[]') {
        return Array.isArray(value) ? value.flatMap(item => resolvePath(item, tail)) : [];
    }
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return [];
    const child = (value as Record<string, unknown>)[head];
    return resolvePath(child, tail);
}

function hasPath(value: unknown, path: string): boolean {
    return resolvePath(value, path.replace(/\[\]/g, '.[]').split('.').filter(Boolean)).length > 0;
}

function valuesAtPath(value: unknown, path: string): unknown[] {
    return resolvePath(value, path.replace(/\[\]/g, '.[]').split('.').filter(Boolean));
}

function matchesType(value: unknown, type: ContractField['type']): boolean {
    if (type === 'array') return Array.isArray(value);
    if (type === 'object') return typeof value === 'object' && value !== null && !Array.isArray(value);
    if (type === 'number') return typeof value === 'number';
    if (type === 'integer') return typeof value === 'number' && Number.isInteger(value);
    return typeof value === type;
}

export function validateSharedFixture(contractName: string): string[] {
    const registry = loadSharedSchemaRegistry();
    const contract = registry.contracts[contractName];
    if (!contract) return [`unknown contract: ${contractName}`];
    const fixture = loadSharedSchemaFixture(contractName);
    if (typeof fixture !== 'object' || fixture === null || Array.isArray(fixture)) return ['fixture root must be an object'];

    const root = fixture as Record<string, unknown>;
    const errors: string[] = [];
    for (const field of contract.required) {
        if (!hasPath(root, field)) errors.push(`missing required field: ${field}`);
    }
    for (const [path, metadata] of Object.entries(contract.fields)) {
        if (!hasPath(root, path)) {
            if (metadata.required) errors.push(`missing required field: ${path}`);
            continue;
        }
        if (metadata.compatibility_alias) continue;
        const values = valuesAtPath(root, path);
        for (const value of values) {
            if (value === null) {
                if (!metadata.nullable) errors.push(`field is not nullable: ${path}`);
                continue;
            }
            if (!matchesType(value, metadata.type)) errors.push(`wrong type for ${path}`);
            if (metadata.enum && (typeof value !== 'string' || !metadata.enum.includes(value))) {
                errors.push(`invalid enum for ${path}: ${String(value)}`);
            }
            if (metadata.finite && typeof value === 'number' && !Number.isFinite(value)) {
                errors.push(`non-finite number for ${path}`);
            }
            if (metadata.format === 'date-time' && typeof value === 'string' && Number.isNaN(Date.parse(value))) {
                errors.push(`invalid date-time for ${path}`);
            }
        }
    }
    return errors;
}
