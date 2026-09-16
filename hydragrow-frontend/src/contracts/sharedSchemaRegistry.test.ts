import { describe, expect, it } from 'vitest';
import {
    loadSharedSchemaFixture,
    loadSharedSchemaRegistry,
    validateSharedFixture,
} from './sharedSchemaRegistry';

describe('P1.9 canonical shared schema registry', () => {
    it('loads a versioned registry with canonical wire contracts', () => {
        const registry = loadSharedSchemaRegistry();
        expect(registry.artifact_kind).toBe('contract_registry_metadata');
        expect(registry.registry_version).toBe('1.0.0');
        expect(registry.schema_dialect).toBe('https://json-schema.org/draft/2020-12/schema');
        expect(Object.keys(registry.contracts).length).toBeGreaterThanOrEqual(8);
        for (const contract of Object.values(registry.contracts)) {
            expect(contract.layer).toBe('canonical_wire');
            expect(contract.version).toEqual(expect.any(Number));
            expect(contract.fixture).toMatch(/^fixtures\/.+\.json$/);
        }
    });

    it('validates every shared fixture against registry metadata', () => {
        const registry = loadSharedSchemaRegistry();
        for (const name of Object.keys(registry.contracts)) {
            expect(validateSharedFixture(name), name).toEqual([]);
        }
    });

    it('keeps canonical aliases input-only', () => {
        const sensor = loadSharedSchemaFixture('sensor_data') as Record<string, unknown>;
        const command = loadSharedSchemaFixture('mqtt_command') as Record<string, unknown>;
        expect(sensor).toHaveProperty('ec');
        expect(sensor).not.toHaveProperty('tds');
        expect(sensor).toHaveProperty('err_ec');
        expect(sensor).not.toHaveProperty('err_tds');
        expect(command).not.toHaveProperty('pump');
        expect(command).not.toHaveProperty('pump_id');
        expect(command).not.toHaveProperty('duration_sec');
        expect(command).not.toHaveProperty('pwm');
    });

    it('matches shared Rust WiFi enum casing and input-only legacy aliases', () => {
        const registry = loadSharedSchemaRegistry();
        const contract = registry.contracts.wifi_secret_action;
        expect(contract.fields.secret_action.enum).toEqual(['Keep', 'Set', 'Clear']);
        expect(registry.contracts.wifi_secret_action).toBeDefined();
        const fixture = loadSharedSchemaFixture('wifi_secret_action') as Record<string, unknown>;
        expect(fixture.secret_action).toBe('Set');
        expect(validateSharedFixture('wifi_secret_action')).toEqual([]);
    });

    it('documents the Rust f32 to JSON-number precision boundary', () => {
        const sensor = loadSharedSchemaRegistry().contracts.sensor_data;
        for (const field of ['ec', 'ph', 'temp', 'water_level']) {
            expect(sensor.fields[field].precision).toContain('binary32');
            expect(sensor.fields[field].finite).toBe(true);
        }
    });

    it('preserves telemetry axis names and units as closed canonical values', () => {
        const telemetry = loadSharedSchemaFixture('authoritative_telemetry') as {
            axes: Array<{ name: string; unit: string }>;
        };
        expect(telemetry.axes.map(axis => axis.name)).toEqual(['ph', 'ec', 'temp', 'water_level']);
        expect(telemetry.axes.map(axis => axis.unit)).toEqual(['pH', 'mS/cm', '°C', 'cm']);
    });

    it('uses distinct observation and receipt timestamps', () => {
        const telemetry = loadSharedSchemaFixture('authoritative_telemetry') as Record<string, unknown>;
        expect(telemetry.observed_at).toBe('2026-05-19T12:00:00Z');
        expect(telemetry.received_at).toBe('2026-05-19T12:00:01Z');
    });
});
